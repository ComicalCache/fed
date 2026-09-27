use std::{path::PathBuf, time::Duration};

use crossterm::event::{KeyCode, KeyEvent};
use piece_table::Slice;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::{
        normal::{command::Command, keymap},
        search,
    },
    protocols::{
        action::ActionCommand, io::IoCommand, mini_buffer::MiniBufferCommand, view::ViewCommand,
    },
    state::{DocId, StateLock, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyNode, Keymap, Motion},
    util,
};

pub struct NormalKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    replace: bool,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    io_tx: UnboundedSender<IoCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    quit_tx: UnboundedSender<()>,
    view_tx: UnboundedSender<ViewCommand>,
}

impl NormalKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        io_tx: UnboundedSender<IoCommand>, mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
        quit_tx: UnboundedSender<()>, view_tx: UnboundedSender<ViewCommand>,
    ) -> Self {
        Self {
            keymap: keymap::keymap(),
            pending_keys: Vec::new(),
            replace: false,
            state_lock,
            action_tx,
            io_tx,
            mini_buffer_tx,
            quit_tx,
            view_tx,
        }
    }

    fn execute(&mut self, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // Click active view, just abort.
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        drop(state);

        match cmd {
            Command::Move(_) | Command::Delete(_) | Command::Change(_) | Command::Yank(_) => {
                self.motion(view, doc, cmd)
            }
            Command::DeleteLine | Command::ChangeLine | Command::YankLine => {
                self.line_operation(view, doc, cmd)
            }
            Command::ScrollView(motion) => self.scroll_view(view, *motion),
            Command::Undo => {
                let _ = self.action_tx.send(ActionCommand::Undo { view });
            }
            Command::HotRedo => {
                let _ = self.action_tx.send(ActionCommand::HotRedo { view });
            }
            Command::Append
            | Command::AppendEndOfLine
            | Command::InsertLineBelow
            | Command::InsertLineAbove => self.insert(view, doc, cmd),
            Command::SwapLineDown | Command::SwapLineUp => self.swap_operation(view, doc, cmd),
            Command::Indent | Command::Dedent => self.line_operation(view, doc, cmd),
            Command::Paste => {
                if let Ok(mut clipboard) = arboard::Clipboard::new()
                    && let Ok(text) = clipboard.get_text()
                {
                    let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
                    let _ = self.action_tx.send(ActionCommand::Insert { view, text });
                    let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
                }
            }
            Command::DeleteChar => {
                let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
                let _ = self.action_tx.send(ActionCommand::Delete { view });
                let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
            }
            Command::Replace => self.replace = true,
            Command::ReplaceChar(ch) => self.replace_char(view, doc, *ch),
            Command::SaveFile => self.save_file(doc),
            Command::Jump => self.jump(view),
            Command::EnterInsertMode => {
                let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
            }
            Command::EnterVisualMode => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Visual });
            }
            Command::EnterSearchMode => search::util::start_search(
                self.state_lock.clone(),
                view,
                doc,
                None,
                self.mini_buffer_tx.clone(),
            ),
            Command::Quit => {
                let _ = self.quit_tx.send(());
            }
        }
    }

    fn motion(&self, view: ViewId, doc: DocId, cmd: &Command) {
        let motion = match cmd {
            Command::Move(motion) => *motion,
            Command::Delete(motion) => *motion,
            Command::Change(motion) => *motion,
            Command::Yank(motion) => *motion,
            _ => unreachable!(),
        };

        if matches!(cmd, Command::Move(_)) {
            let _ =
                self.action_tx.send(ActionCommand::MoveCursors { view, motion, move_anchor: true });
            return;
        }

        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let offsets = util::motion_offsets(motion, vse, dse);
        if offsets.is_empty() {
            return;
        }
        drop(state);

        if matches!(cmd, Command::Yank(_)) {
            let state = self.state_lock.read();
            let Some(dse) = state.doc_store.get(&doc) else {
                debug_panic!();
                return;
            };

            let mut yanked = String::new();
            for &(start, end) in &offsets {
                let s = start.min(end);
                let e = start.max(end);
                yanked.push_str(&dse.doc.data.slice(s..e));

                // Newline to separate multi-cursor yanks.
                yanked.push('\n');
            }
            yanked.pop();

            drop(state);

            if !yanked.is_empty()
                && let Ok(mut clipboard) = arboard::Clipboard::new()
            {
                let _ = clipboard.set_text(yanked);
            }

            return;
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

        if matches!(cmd, Command::Change(_)) || matches!(cmd, Command::Delete(_)) {
            let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
        }

        for (start, end) in merged {
            if end > start {
                let _ = self.action_tx.send(ActionCommand::Remove {
                    view,
                    offset: start,
                    len: end - start,
                });
            }
        }

        if matches!(cmd, Command::Change(_)) {
            let _ = self
                .action_tx
                .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
        } else if matches!(cmd, Command::Delete(_)) {
            let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
        }
    }

    fn line_operation(&self, view: ViewId, doc: DocId, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut lines = Vec::new();
        for cursor in &vse.cursors.list {
            let doc_lines = dse.doc.data.lines();

            let mut y = doc_lines.saturating_sub(1);
            for idx in 0..doc_lines {
                if cursor.offset >= dse.doc.data.get_line_start_byte(idx)
                    && (cursor.offset < dse.doc.data.get_line_end_byte(idx) || idx == doc_lines - 1)
                {
                    y = idx;
                    break;
                }
            }

            lines.push(y);
        }
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return;
        }

        let mut actions = Vec::new();
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

                    actions.push(ActionCommand::Remove { view, offset: start, len: end - start });
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

                    actions.push(ActionCommand::Remove { view, offset: start, len: end - start });
                }

                actions
                    .push(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
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
                    actions.push(ActionCommand::InsertAt {
                        view,
                        text: "\t".to_string(),
                        offset: dse.doc.data.get_line_start_byte(y),
                    });
                }
            }
            Command::Dedent => {
                // FIXME: option to also respect leading spaces.
                for y in lines.into_iter().rev() {
                    let start = dse.doc.data.get_line_start_byte(y);
                    let end = dse.doc.data.get_line_end_byte(y);

                    if dse.doc.data.slice(start..end).starts_with('\t') {
                        actions.push(ActionCommand::Remove { view, offset: start, len: 1 });
                    }
                }
            }
            _ => unreachable!(),
        }
        drop(state);

        if actions.is_empty() {
            return;
        }

        if !matches!(cmd, Command::YankLine) {
            let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
        }
        for action in actions {
            let _ = self.action_tx.send(action);
        }
        if !matches!(cmd, Command::YankLine) && !matches!(cmd, Command::ChangeLine) {
            let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
        }
    }

    fn scroll_view(&self, view: ViewId, motion: Motion) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };

        let mut scroll = *vse.scroll;
        drop(state);

        match motion {
            Motion::Up => scroll.y = scroll.y.saturating_sub(1),
            Motion::Down => scroll.y = scroll.y.saturating_add(1),
            Motion::Left => scroll.x = scroll.x.saturating_sub(1),
            Motion::Right => scroll.x = scroll.x.saturating_add(1),
            _ => {}
        }

        let _ = self.view_tx.send(ViewCommand::ScrollTo { view, pos: scroll });
    }

    fn insert(&self, view: ViewId, doc: DocId, cmd: &Command) {
        let _ = self.action_tx.send(ActionCommand::StartCommit { doc });

        match cmd {
            Command::Append => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion: Motion::Right,
                    move_anchor: true,
                });
            }
            Command::AppendEndOfLine => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion: Motion::EndOfLine,
                    move_anchor: true,
                });
            }
            Command::InsertLineBelow => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion: Motion::EndOfLine,
                    move_anchor: true,
                });
                let _ = self.action_tx.send(ActionCommand::Insert { view, text: "\n".to_string() });
            }
            Command::InsertLineAbove => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion: Motion::BeginningOfLine,
                    move_anchor: true,
                });
                let _ = self.action_tx.send(ActionCommand::Insert { view, text: "\n".to_string() });
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion: Motion::Left,
                    move_anchor: true,
                });
            }
            _ => unreachable!(),
        }

        let _ = self
            .action_tx
            .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
    }

    fn swap_operation(&self, view: ViewId, doc: DocId, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let doc_lines = dse.doc.data.lines();

        let mut lines = Vec::new();
        for cursor in &vse.cursors.list {
            let mut y = doc_lines.saturating_sub(1);
            for idx in 0..doc_lines {
                if cursor.offset >= dse.doc.data.get_line_start_byte(idx)
                    && (cursor.offset < dse.doc.data.get_line_end_byte(idx) || idx == doc_lines - 1)
                {
                    y = idx;
                    break;
                }
            }

            lines.push(y);
        }
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return;
        }

        // Group contiguous lines into blocks and move them together.
        let mut blocks: Vec<(usize, usize)> = Vec::new();
        for &y in &lines {
            if let Some(last) = blocks.last_mut() {
                if last.1 + 1 == y {
                    last.1 = y;
                    continue;
                }
            }

            blocks.push((y, y));
        }

        let mut actions = Vec::new();
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

                        actions.push(ActionCommand::Remove {
                            view,
                            offset: target_start,
                            len: target_end - target_start,
                        });

                        // Remove the newline of not the block since it will be
                        // at the end. This will break for Windows CRLF line
                        // endings.
                        actions.push(ActionCommand::Remove { view, offset: block_end - 1, len: 1 });
                        actions.push(ActionCommand::InsertAt {
                            view,
                            text: target_text,
                            offset: block_start,
                        });
                    } else {
                        actions.push(ActionCommand::Remove {
                            view,
                            offset: target_start,
                            len: target_end - target_start,
                        });
                        actions.push(ActionCommand::InsertAt {
                            view,
                            text: target_text,
                            offset: block_start,
                        });
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

                        actions.push(ActionCommand::Remove {
                            view,
                            offset: target_start,
                            len: target_len,
                        });
                        actions.push(ActionCommand::InsertAt {
                            view,
                            text: "\n".to_string(),
                            offset: block_end - target_len,
                        });
                        actions.push(ActionCommand::InsertAt {
                            view,
                            text: target_text,
                            offset: block_end - target_len + 1,
                        });
                    } else {
                        actions.push(ActionCommand::Remove {
                            view,
                            offset: target_start,
                            len: target_len,
                        });
                        actions.push(ActionCommand::InsertAt {
                            view,
                            text: target_text,
                            offset: block_end - target_len,
                        });
                    }
                }
            }
            _ => {}
        }
        drop(state);

        if actions.is_empty() {
            return;
        }

        let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
        for action in actions {
            let _ = self.action_tx.send(action);
        }
        let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
    }

    fn replace_char(&self, view: ViewId, doc: DocId, ch: char) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut edits = Vec::new();
        let lines = dse.doc.data.lines();
        for cursor in &vse.cursors.list {
            let offset = cursor.offset;
            let mut y = lines.saturating_sub(1);
            for idx in 0..lines {
                if offset >= dse.doc.data.get_line_start_byte(idx)
                    && (offset < dse.doc.data.get_line_end_byte(idx) || idx == lines - 1)
                {
                    y = idx;
                    break;
                }
            }

            let end = dse.doc.data.get_line_end_byte(y);
            if offset < end {
                let text = dse.doc.data.slice(offset..end).to_string();

                // Disallow replacing newline characters.This will break for
                // Windows CRLF line endings.
                if text.starts_with('\n') {
                    continue;
                }

                let len = text.graphemes(true).next().map(|g| g.len()).unwrap_or(1);
                edits.push((offset, len));
            }
        }
        drop(state);

        if edits.is_empty() {
            return;
        }
        edits.sort_unstable_by_key(|&(o, _)| std::cmp::Reverse(o));
        edits.dedup_by_key(|&mut (o, _)| o);

        let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
        for (offset, len) in edits {
            let _ = self.action_tx.send(ActionCommand::Remove { view, offset, len });
            let _ =
                self.action_tx.send(ActionCommand::InsertAt { view, text: ch.to_string(), offset });
        }
        let _ = self.action_tx.send(ActionCommand::MoveCursors {
            view,
            motion: Motion::Left,
            move_anchor: true,
        });
        let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
    }

    fn save_file(&self, doc: DocId) {
        let state = self.state_lock.read();
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let path = dse
            .doc
            .path
            .as_ref()
            .map(|p| p.absolute().map(|p| Some(p.display().to_string())).unwrap_or(None))
            .flatten()
            .unwrap_or_default();
        drop(state);

        let (id_tx, _) = oneshot::channel();
        let (res_tx, res_rx) = oneshot::channel();
        let _ = self.mini_buffer_tx.send(MiniBufferCommand::Prompt {
            prompt: "Save file: ".to_string(),
            initial_text: if path.is_empty() { None } else { Some(path) },
            id_tx,
            res_tx,
        });

        tokio::spawn(Self::save_file_completion(
            self.state_lock.clone(),
            doc,
            res_rx,
            self.io_tx.clone(),
            self.action_tx.clone(),
            self.mini_buffer_tx.clone(),
        ));
    }

    async fn save_file_completion(
        state_lock: StateLock, doc: DocId, res_rx: oneshot::Receiver<String>,
        io_tx: UnboundedSender<IoCommand>, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    ) {
        let Ok(res) = res_rx.await else { return };

        if res.is_empty() {
            return;
        }

        let path = PathBuf::from(res);

        // The scoping is kind of a hack: the compiler incorrectly doesn't
        // acknowledge that the dropped state.. has been dropped and claims that
        // the `rx.await` call following this scope is illegal because the
        // `RwLockReadGuard` is not `Send`.
        let (tx, rx) = oneshot::channel();
        {
            let state = state_lock.read();
            let Some(dse) = state.doc_store.get(&doc) else { return };
            let data = dse.doc.data.slice(0..dse.doc.data.len());
            drop(state);

            let _ = io_tx.send(IoCommand::Write { path: path.clone(), data, tx });
        }

        match rx.await {
            Ok(Ok(())) => {
                let mut state = state_lock.write();
                if let Some(dse) = state.doc_store.get_mut(&doc) {
                    dse.doc.path = Some(path);
                }
                drop(state);

                let _ = action_tx.send(ActionCommand::Saved { doc });

                let (msg_tx, msg_rx) = oneshot::channel();
                let _ = mini_buffer_tx.send(MiniBufferCommand::Message {
                    message: "File saved".to_string(),
                    tx: msg_tx,
                });

                tokio::spawn(async move {
                    let Ok(id) = msg_rx.await else { return };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = mini_buffer_tx.send(MiniBufferCommand::Close { id });
                });
            }
            Ok(Err(err)) => {
                let (msg_tx, msg_rx) = oneshot::channel();
                let _ = mini_buffer_tx.send(MiniBufferCommand::Message {
                    message: format!("Error saving file: {err}"),
                    tx: msg_tx,
                });

                tokio::spawn(async move {
                    let Ok(id) = msg_rx.await else { return };

                    tokio::time::sleep(Duration::from_secs(4)).await;

                    let _ = mini_buffer_tx.send(MiniBufferCommand::Close { id });
                });
            }
            Err(_) => {}
        }
    }

    fn jump(&self, view: ViewId) {
        let (id_tx, _) = oneshot::channel();
        let (res_tx, res_rx) = oneshot::channel();

        let _ = self.mini_buffer_tx.send(MiniBufferCommand::Prompt {
            prompt: "Jump to: ".to_string(),
            initial_text: None,
            id_tx,
            res_tx,
        });

        tokio::spawn(Self::jump_completion(
            self.state_lock.clone(),
            view,
            res_rx,
            self.action_tx.clone(),
        ));
    }

    async fn jump_completion(
        state_lock: StateLock, view: ViewId, res_rx: oneshot::Receiver<String>,
        action_tx: UnboundedSender<ActionCommand>,
    ) {
        let Ok(res) = res_rx.await else { return };
        if res.is_empty() {
            return;
        }

        let parts: Vec<&str> = res.split(':').collect();

        let state = state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(cursor) = vse.cursors.list.first() else {
            return;
        };
        let Some(pos) = state.offset_to_pos(view, cursor.offset) else {
            debug_panic!();
            return;
        };
        drop(state);

        let mut x = pos.x;
        let mut y = pos.y;

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

        let _ = action_tx.send(ActionCommand::MoveCursorToPos {
            view,
            pos: crate::types::Pos::new(x, y),
            move_anchor: true,
        });
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return false;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return false;
        };

        if vse.mode != ViewStoreTypes::Mode::Normal {
            return false;
        }
        drop(state);

        if self.replace {
            if let KeyCode::Char(ch) = event.code {
                let cmd = Command::ReplaceChar(ch);
                self.execute(&cmd);
            }

            self.replace = false;
            self.pending_keys.clear();

            return true;
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        let mut target = None;
        let mut curr = &self.keymap.root;
        for (idx, key_chord) in self.pending_keys.iter().enumerate() {
            if let Some(node) = curr.get(key_chord) {
                if idx == self.pending_keys.len() - 1 {
                    target = Some(node.clone());
                } else if let KeyNode::Prefix(next_map) = node {
                    curr = next_map;
                } else {
                    debug_panic!();
                }
            } else {
                // Invalid sequence.
                self.pending_keys.clear();

                return false;
            }
        }

        match target {
            Some(KeyNode::Leaf(cmd)) => {
                self.execute(&cmd);
                self.pending_keys.clear();

                true
            }
            Some(KeyNode::Prefix(_)) => true,
            None => {
                self.pending_keys.clear();

                false
            }
        }
    }
}
