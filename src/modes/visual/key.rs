use crossterm::event::KeyEvent;
use piece_table::Slice;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::{
        common, search,
        visual::{cmd::Cmd, keymap},
    },
    protocols::action::ActionCmd,
    state::{DocId, State, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct VisualKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl VisualKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), last_view: None }
    }

    fn exec(&mut self, state: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
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
            Cmd::Move(motion) => Some(vec![FCmd::Action(ActionCmd::MoveCursors {
                view,
                motion,
                move_anchor: false,
            })]),
            Cmd::Delete | Cmd::Change => {
                if dse.read_only {
                    return None;
                }

                self.operator(state, view, doc, cmd)
            }
            Cmd::Yank => self.operator(state, view, doc, cmd),
            Cmd::SwapLineDown | Cmd::SwapLineUp => {
                if dse.read_only {
                    return None;
                }

                self.swap_operation(state, view, doc, cmd)
            }
            Cmd::EnterSearchMode => self.enter_search_mode(state, view, doc),
            Cmd::Escape => self.escape(view),
        }
    }

    fn swap_operation(
        &self, state: &State, view: ViewId, doc: DocId, cmd: Cmd,
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

        common::swap_lines(state, view, doc, lines, matches!(cmd, Cmd::SwapLineDown))
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

    fn operator(&self, state: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
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

        if matches!(cmd, Cmd::Yank) {
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

        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];
        cmds.extend(common::remove_offsets(view, offsets));

        if matches!(cmd, Cmd::Change) {
            cmds.push(FCmd::Action(ActionCmd::PopViewMode { view }));
            cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Insert,
            }));
        } else if matches!(cmd, Cmd::Delete) {
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
