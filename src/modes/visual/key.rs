use crossterm::event::KeyEvent;
use piece_table::Slice;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::{
        search,
        visual::{command::Command, keymap},
    },
    protocols::action::ActionCmd,
    state::{DocId, State, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct VisualKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl VisualKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), last_view: None }
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
            Command::Move(motion) => Some(vec![FCmd::Action(ActionCmd::MoveCursors {
                view,
                motion,
                move_anchor: false,
            })]),
            Command::SwapLineDown | Command::SwapLineUp => {
                if dse.read_only {
                    return None;
                }

                self.swap_operation(state, view, doc, cmd)
            }
            Command::EnterSearchMode => self.enter_search_mode(state, view, doc),
            Command::Escape => self.escape(view),
            Command::Yank => self.operator(state, view, doc, cmd),
            Command::Delete | Command::Change => {
                if dse.read_only {
                    return None;
                }

                self.operator(state, view, doc, cmd)
            }
        }
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

        let mut lines = Vec::new();
        for cursor in &vse.cursors.list {
            let start = cursor.anchor.min(cursor.offset);
            let end = cursor.anchor.max(cursor.offset);

            for idx in 0..doc_lines {
                let s = dse.doc.data.get_line_start_byte(idx);
                let e = dse.doc.data.get_line_end_byte(idx);

                if start == end {
                    if start >= s && (start < e || idx == doc_lines - 1) {
                        lines.push(idx);
                    }
                } else if start < e && end > s {
                    lines.push(idx);
                }
            }
        }
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

    fn enter_search_mode(&self, state: &State, view: ViewId, doc: DocId) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let mut offsets = Vec::new();
        for cursor in &vse.cursors.list {
            let start = cursor.anchor.min(cursor.offset);
            let mut end = cursor.anchor.max(cursor.offset);

            let text = dse.doc.data.slice(end..dse.doc.data.len());
            if let Some(grapheme) = text.graphemes(true).next() {
                end += grapheme.len();
            }

            offsets.push((start, end));
        }

        offsets.sort_by_key(|&(s, _)| s);

        let mut bounds: Vec<(usize, usize)> = Vec::new();
        for (start, end) in offsets {
            if let Some(last) = bounds.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }

            bounds.push((start, end));
        }

        Some(search::util::start_search(view, doc, Some(bounds)))
    }

    fn operator(&self, state: &State, view: ViewId, doc: DocId, cmd: Command) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let mut offsets = Vec::new();
        for cursor in &vse.cursors.list {
            let start = cursor.anchor.min(cursor.offset);
            let mut end = cursor.anchor.max(cursor.offset);

            let text = dse.doc.data.slice(end..dse.doc.data.len());
            if let Some(grapheme) = text.graphemes(true).next() {
                end += grapheme.len();
            }

            offsets.push((start, end));
        }

        if matches!(cmd, Command::Yank) {
            let mut yanked = String::new();
            for &(start, end) in &offsets {
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

            return Some(vec![FCmd::Action(ActionCmd::PopViewMode { view })]);
        }

        offsets.sort_by_key(|&(s, _)| s);

        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in offsets {
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

        if matches!(cmd, Command::Change) || matches!(cmd, Command::Delete) {
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

        if matches!(cmd, Command::Change) {
            cmds.push(FCmd::Action(ActionCmd::PopViewMode { view }));
            cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Insert,
            }));
        } else if matches!(cmd, Command::Delete) {
            cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
            cmds.push(FCmd::Action(ActionCmd::PopViewMode { view }));
        }

        Some(cmds)
    }

    fn escape(&self, view: ViewId) -> Option<Vec<FCmd>> {
        Some(vec![
            FCmd::Action(ActionCmd::ResetCursorAnchors { view }),
            FCmd::Action(ActionCmd::PopViewMode { view }),
        ])
    }
}

impl KeyInputHandler for VisualKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::VisualMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return None;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };

        if vse.mode() != ViewStoreTypes::Mode::Visual {
            return None;
        }

        if self.last_view != Some(view) {
            self.pending_keys.clear();

            self.last_view = Some(view);
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
