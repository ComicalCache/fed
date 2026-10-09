use std::{path::PathBuf, time::Duration};

use crossterm::event::{KeyCode, KeyEvent};
use piece_table::Slice;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::{
        normal::{command::Command, keymap},
        search,
    },
    protocols::{
        action::ActionCmd,
        dir::DirCmd,
        io::{IoCmd, IoFuture},
        mp::MpCmd,
        view::ViewCmd,
    },
    state::{DocId, State, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, Motion, ParseResult, Pos},
    util,
};

pub struct NormalKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    replace: bool,

    last_view: Option<ViewId>,
}

impl NormalKeyInput {
    pub fn new() -> Self {
        Self { keymap: keymap::keymap(), pending_keys: Vec::new(), replace: false, last_view: None }
    }

    fn exec(&mut self, state: &State, cmd: Command) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else { return None };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return None;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return None;
        };

        match cmd {
            Command::Yank(_) | Command::Move(_) => self.motion(state, view, doc, cmd),
            Command::Delete(_) | Command::Change(_) => {
                if dse.read_only {
                    return None;
                }

                self.motion(state, view, doc, cmd)
            }
            Command::YankLine => self.line_operation(state, view, doc, cmd),
            Command::DeleteLine | Command::ChangeLine => {
                if dse.read_only {
                    return None;
                }

                self.line_operation(state, view, doc, cmd)
            }
            Command::ScrollView(motion) => self.scroll_view(state, view, motion),
            Command::Undo => {
                if dse.read_only {
                    return None;
                }

                Some(vec![FCmd::Action(ActionCmd::Undo { view })])
            }
            Command::HotRedo => {
                if dse.read_only {
                    return None;
                }

                Some(vec![FCmd::Action(ActionCmd::HotRedo { view })])
            }
            Command::Append
            | Command::AppendEndOfLine
            | Command::InsertLineBelow
            | Command::InsertLineAbove => {
                if dse.read_only {
                    return None;
                }

                self.insert(view, doc, cmd)
            }
            Command::SwapLineDown | Command::SwapLineUp => {
                if dse.read_only {
                    return None;
                }

                self.swap_operation(state, view, doc, cmd)
            }
            Command::Indent | Command::Dedent => {
                if dse.read_only {
                    return None;
                }

                self.line_operation(state, view, doc, cmd)
            }
            Command::Paste => {
                if !dse.read_only
                    && let Ok(mut clipboard) = arboard::Clipboard::new()
                    && let Ok(text) = clipboard.get_text()
                {
                    Some(vec![
                        FCmd::Action(ActionCmd::StartCommit { doc }),
                        FCmd::Action(ActionCmd::Insert { view, text }),
                        FCmd::Action(ActionCmd::EndCommit { doc }),
                    ])
                } else {
                    None
                }
            }
            Command::DeleteChar => {
                if dse.read_only {
                    return None;
                }

                Some(vec![
                    FCmd::Action(ActionCmd::StartCommit { doc }),
                    FCmd::Action(ActionCmd::Delete { view }),
                    FCmd::Action(ActionCmd::EndCommit { doc }),
                ])
            }
            Command::Replace => {
                if dse.read_only {
                    return None;
                }

                self.replace = true;

                None
            }
            Command::ReplaceChar(ch) => {
                if dse.read_only {
                    return None;
                }

                self.replace_char(state, view, doc, ch)
            }
            Command::SaveFile => self.save_file(state, doc),
            Command::Jump => self.jump(state, view),
            Command::EnterInsertMode => {
                if dse.read_only {
                    return None;
                }

                Some(vec![
                    FCmd::Action(ActionCmd::StartCommit { doc }),
                    FCmd::Action(ActionCmd::PushViewMode {
                        view,
                        mode: ViewStoreTypes::Mode::Insert,
                    }),
                ])
            }
            Command::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Command::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Command::EnterDirMode => {
                let Some(window) = state.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Dir(DirCmd::ReplaceWindow { window })])
            }
            Command::Quit => Some(vec![FCmd::Quit]),
        }
    }

    fn motion(&self, state: &State, view: ViewId, doc: DocId, cmd: Command) -> Option<Vec<FCmd>> {
        let motion = match cmd {
            Command::Move(motion) => motion,
            Command::Delete(motion) => motion,
            Command::Change(motion) => motion,
            Command::Yank(motion) => motion,
            _ => unreachable!(),
        };

        if matches!(cmd, Command::Move(_)) {
            return Some(vec![FCmd::Action(ActionCmd::MoveCursors {
                view,
                motion,
                move_anchor: true,
            })]);
        }

        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let offsets = util::motion::offsets(motion, vse, dse);
        if offsets.is_empty() {
            return None;
        }

        if matches!(cmd, Command::Yank(_)) {
            let mut yanked = String::new();
            for &(start, end) in &offsets {
                let s = start.min(end);
                let e = start.max(end);
                yanked.push_str(&dse.doc.data.slice(s..e));

                // Newline to separate multi-cursor yanks.
                yanked.push('\n');
            }
            yanked.pop();

            if !yanked.is_empty()
                && let Ok(mut clipboard) = arboard::Clipboard::new()
            {
                let _ = clipboard.set_text(yanked);
            }

            return Some(Vec::new());
        }

        let mut normalized: Vec<_> =
            offsets.into_iter().map(|(s, e)| (s.min(e), s.max(e))).collect();
        normalized.sort_by_key(|&(s, _)| s);

        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in normalized {
            if let Some(last) = merged.last_mut()
                && start <= last.1
            {
                last.1 = last.1.max(end);
                continue;
            }

            merged.push((start, end));
        }
        merged.sort_by_key(|&(s, _)| std::cmp::Reverse(s));

        let mut cmds = Vec::new();
        if matches!(cmd, Command::Change(_)) || matches!(cmd, Command::Delete(_)) {
            cmds.push(FCmd::Action(ActionCmd::StartCommit { doc }));
        }

        for (start, end) in merged {
            if end > start {
                cmds.push(FCmd::Action(ActionCmd::Remove {
                    view,
                    offset: start,
                    len: end - start,
                }));
            }
        }

        if matches!(cmd, Command::Change(_)) {
            cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Insert,
            }));
        } else if matches!(cmd, Command::Delete(_)) {
            cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
        }

        Some(cmds)
    }

    fn line_operation(
        &self, state: &State, view: ViewId, doc: DocId, cmd: Command,
    ) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let mut lines: Vec<_> =
            vse.cursors.list.iter().map(|c| dse.doc.data.get_line_of_byte(c.offset)).collect();
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return None;
        }

        let mut cmds = Vec::new();
        match cmd {
            Command::DeleteLine => {
                for y in lines.into_iter().rev() {
                    let mut start = dse.doc.data.get_line_start_byte(y);
                    let end = dse.doc.data.get_line_end_byte(y);

                    // When deleting the last line, delete the newline from the
                    // previous line. This will break for Windows CRLF line
                    // endings.
                    if y + 1 == dse.doc.data.lines() {
                        start = start.saturating_sub(1);
                    }

                    cmds.push(FCmd::Action(ActionCmd::Remove {
                        view,
                        offset: start,
                        len: end - start,
                    }));
                }
            }
            Command::ChangeLine => {
                for y in lines.into_iter().rev() {
                    let start = dse.doc.data.get_line_start_byte(y);
                    let mut end = dse.doc.data.get_line_end_byte(y);

                    // Remove the newline of not the last line. This will break
                    // for Windows CRLF line endings.
                    if y + 1 != dse.doc.data.lines() {
                        end -= 1;
                    }

                    cmds.push(FCmd::Action(ActionCmd::Remove {
                        view,
                        offset: start,
                        len: end - start,
                    }));
                }

                cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                    view,
                    mode: ViewStoreTypes::Mode::Insert,
                }));
            }
            Command::YankLine => {
                let mut yanked = String::new();
                for &y in &lines {
                    let start = dse.doc.data.get_line_start_byte(y);
                    let mut end = dse.doc.data.get_line_end_byte(y);

                    // Remove the newline of not the last line. This will break
                    // for Windows CRLF line endings.
                    if y + 1 != dse.doc.data.lines() {
                        end -= 1;
                    }

                    yanked.push_str(&dse.doc.data.slice(start..end));

                    // Newline to separate multi-cursor yanks.
                    yanked.push('\n');
                }
                yanked.pop();

                if !yanked.is_empty()
                    && let Ok(mut clipboard) = arboard::Clipboard::new()
                {
                    let _ = clipboard.set_text(yanked);
                }
            }
            Command::Indent => {
                // FIXME: option to convert tab to spaces.
                for y in lines.into_iter().rev() {
                    cmds.push(FCmd::Action(ActionCmd::InsertAt {
                        view,
                        text: "\t".to_string(),
                        offset: dse.doc.data.get_line_start_byte(y),
                    }));
                }
            }
            Command::Dedent => {
                // FIXME: option to also respect leading spaces.
                for y in lines.into_iter().rev() {
                    let start = dse.doc.data.get_line_start_byte(y);
                    let end = dse.doc.data.get_line_end_byte(y);

                    if dse.doc.data.slice(start..end).starts_with('\t') {
                        cmds.push(FCmd::Action(ActionCmd::Remove { view, offset: start, len: 1 }));
                    }
                }
            }
            _ => unreachable!(),
        }

        if cmds.is_empty() {
            return None;
        }

        if !matches!(cmd, Command::YankLine) {
            cmds.insert(0, FCmd::Action(ActionCmd::StartCommit { doc }));
        }
        if !matches!(cmd, Command::YankLine) && !matches!(cmd, Command::ChangeLine) {
            cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
        }

        Some(cmds)
    }

    fn scroll_view(&self, state: &State, view: ViewId, motion: Motion) -> Option<Vec<FCmd>> {
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };

        let mut scroll = *vse.scroll;

        match motion {
            Motion::Up => scroll.y = scroll.y.saturating_sub(1),
            Motion::Down => scroll.y = scroll.y.saturating_add(1),
            Motion::Left => scroll.x = scroll.x.saturating_sub(1),
            Motion::Right => scroll.x = scroll.x.saturating_add(1),
            _ => {}
        }

        Some(vec![FCmd::View(ViewCmd::ScrollTo { view, pos: scroll })])
    }

    fn insert(&self, view: ViewId, doc: DocId, cmd: Command) -> Option<Vec<FCmd>> {
        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];

        match cmd {
            Command::Append => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::Right,
                    move_anchor: true,
                }));
            }
            Command::AppendEndOfLine => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::EndOfLine,
                    move_anchor: true,
                }));
            }
            Command::InsertLineBelow => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::EndOfLine,
                    move_anchor: true,
                }));
                cmds.push(FCmd::Action(ActionCmd::Insert { view, text: "\n".to_string() }));
            }
            Command::InsertLineAbove => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::BeginningOfLine,
                    move_anchor: true,
                }));
                cmds.push(FCmd::Action(ActionCmd::Insert { view, text: "\n".to_string() }));
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::Left,
                    move_anchor: true,
                }));
            }
            _ => unreachable!(),
        }

        cmds.push(FCmd::Action(ActionCmd::PushViewMode {
            view,
            mode: ViewStoreTypes::Mode::Insert,
        }));

        Some(cmds)
    }

    fn swap_operation(
        &self, state: &State, view: ViewId, doc: DocId, cmd: Command,
    ) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let doc_lines = dse.doc.data.lines();

        let mut lines: Vec<_> =
            vse.cursors.list.iter().map(|c| dse.doc.data.get_line_of_byte(c.offset)).collect();
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return None;
        }

        // Group contiguous lines into blocks and move them together.
        let mut blocks: Vec<(usize, usize)> = Vec::new();
        for &y in &lines {
            if let Some(last) = blocks.last_mut()
                && last.1 + 1 == y
            {
                last.1 = y;
                continue;
            }

            blocks.push((y, y));
        }

        let mut cmds = Vec::new();
        match cmd {
            Command::SwapLineDown => {
                for &(start, end) in blocks.iter().rev() {
                    if end + 1 >= doc_lines {
                        continue;
                    }

                    let block_start = dse.doc.data.get_line_start_byte(start);
                    let block_end = dse.doc.data.get_line_end_byte(end);

                    let target = end + 1;
                    let target_start = dse.doc.data.get_line_start_byte(target);
                    let target_end = dse.doc.data.get_line_end_byte(target);
                    let target_text = dse.doc.data.slice(target_start..target_end);

                    // Target line is the last line of the document.
                    if target + 1 == doc_lines {
                        let mut target_text = target_text;
                        target_text.push('\n');

                        cmds.push(FCmd::Action(ActionCmd::Remove {
                            view,
                            offset: target_start,
                            len: target_end - target_start,
                        }));

                        // Remove the newline of not the block since it will be
                        // at the end. This will break for Windows CRLF line
                        // endings.
                        cmds.push(FCmd::Action(ActionCmd::Remove {
                            view,
                            offset: block_end - 1,
                            len: 1,
                        }));
                        cmds.push(FCmd::Action(ActionCmd::InsertAt {
                            view,
                            text: target_text,
                            offset: block_start,
                        }));
                    } else {
                        cmds.push(FCmd::Action(ActionCmd::Remove {
                            view,
                            offset: target_start,
                            len: target_end - target_start,
                        }));
                        cmds.push(FCmd::Action(ActionCmd::InsertAt {
                            view,
                            text: target_text,
                            offset: block_start,
                        }));
                    }
                }
            }
            Command::SwapLineUp => {
                for &(start, end) in blocks.iter().rev() {
                    if start == 0 {
                        continue;
                    }

                    let block_end = dse.doc.data.get_line_end_byte(end);

                    let target = start - 1;
                    let target_start = dse.doc.data.get_line_start_byte(target);
                    let target_end = dse.doc.data.get_line_end_byte(target);
                    let target_text = dse.doc.data.slice(target_start..target_end);
                    let target_len = target_text.len();

                    // Block is the last line(s) of the document.
                    if end + 1 == doc_lines {
                        // Remove the newline of not block since it wont be
                        // at the end. This will break for Windows CRLF line
                        // endings.
                        let mut target_text = target_text;
                        target_text.truncate(target_text.len() - 1);

                        cmds.push(FCmd::Action(ActionCmd::Remove {
                            view,
                            offset: target_start,
                            len: target_len,
                        }));
                        cmds.push(FCmd::Action(ActionCmd::InsertAt {
                            view,
                            text: "\n".to_string(),
                            offset: block_end - target_len,
                        }));
                        cmds.push(FCmd::Action(ActionCmd::InsertAt {
                            view,
                            text: target_text,
                            offset: block_end - target_len + 1,
                        }));
                    } else {
                        cmds.push(FCmd::Action(ActionCmd::Remove {
                            view,
                            offset: target_start,
                            len: target_len,
                        }));
                        cmds.push(FCmd::Action(ActionCmd::InsertAt {
                            view,
                            text: target_text,
                            offset: block_end - target_len,
                        }));
                    }
                }
            }
            _ => {}
        }

        if cmds.is_empty() {
            return None;
        }

        cmds.insert(0, FCmd::Action(ActionCmd::StartCommit { doc }));
        cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));

        Some(cmds)
    }

    fn replace_char(&self, state: &State, view: ViewId, doc: DocId, ch: char) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let mut edits = Vec::new();
        for cursor in &vse.cursors.list {
            let end = dse.doc.data.get_line_end_byte(dse.doc.data.get_line_of_byte(cursor.offset));

            if cursor.offset < end {
                let text = dse.doc.data.slice(cursor.offset..end);

                // Disallow replacing newline characters.This will break for
                // Windows CRLF line endings.
                if text.starts_with('\n') {
                    continue;
                }

                let len = text.graphemes(true).next().map(|g| g.len()).unwrap_or(1);
                edits.push((cursor.offset, len));
            }
        }

        if edits.is_empty() {
            return None;
        }
        edits.sort_unstable_by_key(|&(o, _)| std::cmp::Reverse(o));
        edits.dedup_by_key(|&mut (o, _)| o);

        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];
        for (offset, len) in edits {
            cmds.push(FCmd::Action(ActionCmd::Remove { view, offset, len }));
            cmds.push(FCmd::Action(ActionCmd::InsertAt { view, text: ch.to_string(), offset }));
        }
        cmds.push(FCmd::Action(ActionCmd::MoveCursors {
            view,
            motion: Motion::Left,
            move_anchor: true,
        }));
        cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));

        Some(cmds)
    }

    fn save_file(&self, state: &State, doc: DocId) -> Option<Vec<FCmd>> {
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return None;
        };

        let path = dse.doc.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| {
            format!(
                "{}{}",
                std::env::current_dir().unwrap_or_default().display(),
                std::path::MAIN_SEPARATOR
            )
        });

        let pcallback = Box::new(move |state: &State, res: String| {
            if res.is_empty() {
                return;
            }

            let Some(dse) = state.doc_store.get(&doc) else {
                debug_panic!();
                return;
            };

            let path = util::path::normalize(res);
            let data = dse.doc.data.slice(0..dse.doc.data.len());

            let cb_path = path.clone();
            let async_fcmd_tx = state.async_fcmd_tx.clone();
            let wcallback = Box::new(move |res| -> IoFuture {
                Box::pin(Self::save_file_completion(async_fcmd_tx, res, doc, cb_path))
            });

            let _ = state.fcmd_tx.send(FCmd::Io(IoCmd::Write { path, data, wcallback }));
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "Save file: ".to_string(),
            initial_text: if path.is_empty() { None } else { Some(path) },
            pcallback,
            tx: None,
        })])
    }

    async fn save_file_completion(
        async_fcmd_tx: UnboundedSender<FCmd>, res: Result<(), String>, doc: DocId, path: PathBuf,
    ) {
        match res {
            Ok(()) => {
                let _ = async_fcmd_tx
                    .send(FCmd::Action(ActionCmd::SetDocPath { doc, path: Some(path) }));
                let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::Saved { doc }));

                let (tx, rx) = oneshot::channel();
                let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Message {
                    message: "File saved".to_string(),
                    tx: Some(tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });
            }
            Err(err) => {
                let (tx, rx) = oneshot::channel();
                let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Message {
                    message: format!("Error saving file: {err}"),
                    tx: Some(tx),
                }));

                tokio::spawn(async move {
                    let Ok(id) = rx.await else {
                        debug_panic!();
                        return;
                    };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                });
            }
        }
    }

    fn jump(&self, state: &State, view: ViewId) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };
        let Some(cursor) = vse.cursors.list.first() else {
            return None;
        };

        let pos = util::offset_to_pos(cursor.offset, vse, dse);

        let mut x = pos.x;
        let mut y = pos.y;

        let pcallback = Box::new(move |state: &State, res: String| {
            if res.is_empty() {
                return;
            }

            let parts: Vec<&str> = res.split(':').collect();

            if parts.len() > 0 && !parts[0].is_empty() {
                if let Ok(val) = parts[0].parse::<usize>() {
                    y = val.saturating_sub(1);
                }
            }
            if parts.len() > 1 && !parts[1].is_empty() {
                if let Ok(val) = parts[1].parse::<usize>() {
                    x = val.saturating_sub(1);
                }
            }

            let _ = state.fcmd_tx.send(FCmd::Action(ActionCmd::MoveCursorToPos {
                view,
                pos: Pos::new(x, y),
                move_anchor: true,
            }));
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "Jump to: ".to_string(),
            initial_text: None,
            pcallback,
            tx: None,
        })])
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return None;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };

        if vse.mode() != ViewStoreTypes::Mode::Normal {
            return None;
        }

        if self.last_view != Some(view) {
            self.replace = false;
            self.pending_keys.clear();

            self.last_view = Some(view);
        }

        if self.replace {
            let cmds = if let KeyCode::Char(ch) = event.code {
                // Always treat entering execute as consuming the key.
                Some(self.exec(state, Command::ReplaceChar(ch)).unwrap_or_else(|| Vec::new()))
            } else {
                None
            };

            self.replace = false;
            self.pending_keys.clear();

            return cmds;
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                // Always treat entering execute as consuming the key.
                let cmds = Some(self.exec(state, cmd).unwrap_or_else(|| Vec::new()));
                self.pending_keys.clear();

                cmds
            }
            ParseResult::Prefix => Some(Vec::new()),
            ParseResult::Invalid => {
                self.pending_keys.clear();

                Some(Vec::new())
            }
        }
    }
}
