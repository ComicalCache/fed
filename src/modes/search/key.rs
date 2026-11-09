use crossterm::event::KeyEvent;

use crate::fed::FCmd;
use crate::modes::common;
use crate::modes::search::cmd::Cmd;
use crate::modes::search::keymap;
use crate::protocols::action::ActionCmd;
use crate::protocols::dir::DirCmd;
use crate::protocols::manage::ManageCmd;
use crate::protocols::search::SearchCmd;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::state::ViewStoreTypes::SearchMatch;
use crate::types::KeyChord;
use crate::types::KeyInputHandler;
use crate::types::KeyInputPriority;
use crate::types::Keymap;
use crate::types::ParseResult;

pub struct SearchKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl SearchKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), last_view: None }
    }

    fn exec(s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get(&doc).unwrap();

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(s, view, motion),
            Cmd::YankLine => common::yank_line(s, view),
            Cmd::ScrollView(motion) => Some(common::scroll_view(s, view, motion)),
            Cmd::NextMatch => Some(vec![FCmd::Search(SearchCmd::Navigate { view, forward: true })]),
            Cmd::PrevMatch => {
                Some(vec![FCmd::Search(SearchCmd::Navigate { view, forward: false })])
            }
            Cmd::CursorsBegin => Self::cursors(s, view, true),
            Cmd::CursorsEnd => Self::cursors(s, view, false),
            Cmd::Replace | Cmd::ReplaceAll => {
                if dse.read_only {
                    return None;
                }

                Some(vec![FCmd::Search(SearchCmd::Replace {
                    view,
                    doc,
                    all: matches!(cmd, Cmd::ReplaceAll),
                })])
            }
            Cmd::SaveFile => {
                if dse.read_only {
                    return None;
                }

                Some(common::save_file(s, doc))
            }
            Cmd::FindFile => Some(common::find_file(s)),
            Cmd::SelectTheme => Some(common::select_theme()),
            Cmd::Jump => common::jump(s, view, true),
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

    fn cursors(s: &State, view: ViewId, front: bool) -> Option<Vec<FCmd>> {
        let mut cmds = Vec::new();

        let (matches, offsets) = Self::search_state(s, view)?;
        cmds.push(FCmd::Action(ActionCmd::RemoveCursors { view, offsets }));
        let offsets = matches.iter().map(|m| if front { m.start } else { m.end }).collect();
        cmds.push(FCmd::Action(ActionCmd::CreateCursorsAtOffsets { view, offsets }));

        cmds.extend(Self::escape(view));

        Some(cmds)
    }

    fn escape(view: ViewId) -> Vec<FCmd> { vec![FCmd::Search(SearchCmd::Close { view })] }

    fn search_state(s: &State, view: ViewId) -> Option<(&[SearchMatch], Vec<usize>)> {
        let vse = s.view_store.get(&view).unwrap();
        let search = vse.search.as_ref()?;

        if search.matches.is_empty() {
            return None;
        }

        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();

        Some((&search.matches, offsets))
    }
}

impl KeyInputHandler for SearchKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::SearchMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let vse = s.view_store.get(&view).unwrap();

        if vse.mode() != ViewStoreTypes::Mode::Search {
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
