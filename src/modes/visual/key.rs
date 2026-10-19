use crossterm::event::KeyEvent;
use piece_table::Slice;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    fed::FCmd,
    modes::{
        common, search,
        visual::{VisualPasteInput, cmd::Cmd, keymap},
    },
    protocols::action::ActionCmd,
    state::{DocId, State, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, ParseResult, PasteInputHandler},
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

    fn exec(s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get(&doc).unwrap();

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

                Some(Self::operator(s, view, doc, cmd))
            }
            Cmd::Yank => Some(Self::operator(s, view, doc, cmd)),
            Cmd::Paste => {
                if dse.read_only {
                    return None;
                }

                VisualPasteInput {}.paste(s, &arboard::Clipboard::new().ok()?.get_text().ok()?)
            }
            Cmd::SwapLineDown | Cmd::SwapLineUp => {
                if dse.read_only {
                    return None;
                }

                Self::swap_operation(s, view, doc, cmd)
            }
            Cmd::EnterSearchMode => Some(Self::enter_search_mode(s, view, doc)),
            Cmd::Escape => Some(Self::escape(view)),
        }
    }

    fn swap_operation(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

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

        common::swap_lines(s, view, doc, lines, matches!(cmd, Cmd::SwapLineDown))
    }

    fn enter_search_mode(s: &State, view: ViewId, doc: DocId) -> Vec<FCmd> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

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
            if let Some(last) = bounds.last_mut()
                && start <= last.1
            {
                last.1 = last.1.max(end);
                continue;
            }

            bounds.push((start, end));
        }

        search::util::start_search(view, doc, Some(bounds))
    }

    fn operator(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Vec<FCmd> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

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

            return Self::escape(view);
        }

        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];
        cmds.extend(common::remove_offsets(view, offsets));

        if matches!(cmd, Cmd::Change) {
            cmds.extend(Self::escape(view));
            cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Insert,
            }));
        } else if matches!(cmd, Cmd::Delete) {
            cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
            cmds.extend(Self::escape(view));
        }

        cmds
    }

    fn escape(view: ViewId) -> Vec<FCmd> {
        vec![
            FCmd::Action(ActionCmd::ResetCursorAnchors { view }),
            FCmd::Action(ActionCmd::PopViewMode { view }),
        ]
    }
}

impl KeyInputHandler for VisualKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::VisualMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let vse = s.view_store.get(&view).unwrap();

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
                let cmds = Some(Self::exec(s, cmd).unwrap_or_default());
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
