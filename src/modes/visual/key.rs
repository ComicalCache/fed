use std::time::Duration;

use crossterm::event::KeyEvent;

use crate::fed::FCmd;
use crate::modes::common;
use crate::modes::visual::VisualPasteInput;
use crate::modes::visual::align;
use crate::modes::visual::cmd::Cmd;
use crate::modes::visual::keymap;
use crate::piece_table::Chunks;
use crate::protocols::action::ActionCmd;
use crate::protocols::dir::DirCmd;
use crate::protocols::grep::GrepCmd;
use crate::protocols::manage::ManageCmd;
use crate::protocols::search::SearchCmd;
use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::KeyChord;
use crate::types::KeyInputHandler;
use crate::types::KeyInputPriority;
use crate::types::Keymap;
use crate::types::ParseResult;
use crate::types::PasteInputHandler;
use crate::util;

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
            Cmd::ScrollView(motion) => Some(common::scroll_view(s, view, motion)),
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
            Cmd::SwapLineDown
            | Cmd::SwapLineUp
            | Cmd::Indent
            | Cmd::Dedent
            | Cmd::Sort
            | Cmd::SortReverse
            | Cmd::Align
            | Cmd::AlignRepeated => {
                if dse.read_only {
                    return None;
                }

                Self::line_operation(s, view, doc, cmd)
            }
            Cmd::EnterVisualMode | Cmd::EnterVisualLineMode => {
                let mode = if matches!(cmd, Cmd::EnterVisualLineMode) {
                    ViewStoreTypes::Mode::VisualLine
                } else {
                    ViewStoreTypes::Mode::Visual
                };

                if s.view_store.get(&view).unwrap().mode() == mode {
                    return Some(Self::escape(view));
                }

                Some(vec![
                    FCmd::Action(ActionCmd::PopViewMode { view }),
                    FCmd::Action(ActionCmd::PushViewMode { view, mode }),
                ])
            }
            Cmd::EnterSearchMode => Some(Self::enter_search_mode(s, view, doc)),
            Cmd::EnterGrepMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Grep(GrepCmd::Search { window })])
            }
            Cmd::SaveFile => {
                if dse.read_only {
                    return None;
                }

                Some(common::save_file(s, doc))
            }
            Cmd::FindFile => Some(common::find_file(s)),
            Cmd::SelectTheme => Some(common::select_theme()),
            Cmd::Jump => common::jump(s, view, false),
            Cmd::FocusWindow(motion) => common::focus_window(s, motion),
            Cmd::EnterDirMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Dir(DirCmd::Show { window, path: None })])
            }
            Cmd::EnterManageMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Manage(ManageCmd::Show { window })])
            }
            Cmd::Quit => common::quit(s, view),
            Cmd::Escape => Some(Self::escape(view)),
        }
    }

    fn line_operation(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let mut lines = Vec::new();
        for cursor in &vse.cursors.list {
            let (start, end) = util::selection::lines(cursor, dse);

            if vse.mode() == ViewStoreTypes::Mode::VisualLine
                || matches!(cmd, Cmd::Sort | Cmd::SortReverse | Cmd::Align | Cmd::AlignRepeated)
            {
                lines.extend(start..=end);
            } else {
                let first = cursor.anchor.min(cursor.offset);
                let last = cursor.anchor.max(cursor.offset);

                for y in start..=end {
                    let s = dse.doc.data.line_start_offset(y);
                    let e = dse.doc.data.line_end_offset(y);

                    if first == last || first < e && last > s {
                        lines.push(y);
                    }
                }
            }
        }

        match cmd {
            Cmd::SwapLineDown | Cmd::SwapLineUp => {
                common::swap_lines(s, view, doc, lines, matches!(cmd, Cmd::SwapLineDown))
            }
            Cmd::Indent | Cmd::Dedent => {
                common::indentation(s, view, lines, matches!(cmd, Cmd::Dedent))
            }
            Cmd::Sort | Cmd::SortReverse => {
                common::sort_lines(s, view, lines, matches!(cmd, Cmd::SortReverse))
            }
            Cmd::Align | Cmd::AlignRepeated => {
                Some(align::prompt(s, view, lines, matches!(cmd, Cmd::AlignRepeated)))
            }
            _ => unreachable!(),
        }
    }

    fn enter_search_mode(s: &State, view: ViewId, doc: DocId) -> Vec<FCmd> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let mut offsets = util::selection::offsets(vse, dse);
        offsets.sort_unstable_by_key(|&(s, _)| s);

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

        vec![FCmd::Search(SearchCmd::Search { view, doc, bounds: Some(bounds) })]
    }

    fn operator(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Vec<FCmd> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let linewise = vse.mode() == ViewStoreTypes::Mode::VisualLine;
        let has_last_line = linewise && util::selection::has_last_line(vse, dse);
        let mut offsets = util::selection::offsets(vse, dse);

        if matches!(cmd, Cmd::Yank) {
            let mut yanked = String::new();
            for &(start, end) in &offsets {
                for chunk in dse.doc.data.chunks(start..end) {
                    yanked.push_str(chunk);
                }

                // Newline to separate multi-cursor yanks.
                if !linewise
                    || !yanked.ends_with('\n')
                    || end == dse.doc.data.len() && has_last_line
                {
                    yanked.push('\n');
                }
            }
            if !linewise {
                yanked.pop();
            }

            if !yanked.is_empty()
                && let Ok(mut clipboard) = arboard::Clipboard::new()
                && clipboard.set_text(yanked).is_ok()
            {
                util::mp::async_message(
                    s.async_fcmd_tx.clone(),
                    "Yanked to clipboard".to_string(),
                    Duration::from_secs(3),
                );
            }

            return Self::escape(view);
        }

        if linewise {
            for (start, end) in &mut offsets {
                if matches!(cmd, Cmd::Change) {
                    // Keep an empty line for the replacement.
                    if *end > *start && (*end < dse.doc.data.len() || !has_last_line) {
                        *end -= 1;
                    }
                } else if *start > 0 && *end == dse.doc.data.len() && has_last_line {
                    *start -= 1;
                }
            }
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

    fn keymap(&self) -> crate::types::KeymapHelp { self.keymap.help("visual") }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let vse = s.view_store.get(&view).unwrap();

        if !matches!(vse.mode(), ViewStoreTypes::Mode::Visual | ViewStoreTypes::Mode::VisualLine) {
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
