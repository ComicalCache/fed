use crossterm::event::KeyEvent;

use crate::fed::FCmd;
use crate::modes::common;
use crate::modes::grep::cmd::Cmd;
use crate::modes::grep::keymap;
use crate::protocols::action::ActionCmd;
use crate::protocols::dir::DirCmd;
use crate::protocols::grep::GrepCmd;
use crate::protocols::manage::ManageCmd;
use crate::protocols::search::SearchCmd;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewStoreTypes;
use crate::types::KeyChord;
use crate::types::KeyInputHandler;
use crate::types::KeyInputPriority;
use crate::types::Keymap;
use crate::types::ParseResult;

pub struct GrepKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,
}

impl GrepKeyInput {
    pub fn new() -> Self { Self { keymap: keymap::keymap(), pending_keys: Vec::new() } }

    fn exec(s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.p.grep.view;

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(s, view, motion),
            Cmd::YankLine => common::yank_line(s, view),
            Cmd::ScrollView(motion) => Some(common::scroll_view(s, view, motion)),
            Cmd::NextMatch => Some(vec![FCmd::Grep(GrepCmd::Navigate { forward: true })]),
            Cmd::PrevMatch => Some(vec![FCmd::Grep(GrepCmd::Navigate { forward: false })]),
            Cmd::Replace => Some(vec![FCmd::Grep(GrepCmd::Replace { all: false })]),
            Cmd::ReplaceAll => Some(vec![FCmd::Grep(GrepCmd::Replace { all: true })]),
            Cmd::SaveAll => Some(vec![FCmd::Grep(GrepCmd::SaveAll)]),
            Cmd::FindFile => Some(common::find_file(s)),
            Cmd::SelectTheme => Some(common::select_theme()),
            Cmd::Jump => common::jump(s, view, true),
            Cmd::Select => Some(vec![FCmd::Grep(GrepCmd::SelectResult)]),
            Cmd::FocusWindow(motion) => common::focus_window(s, motion),
            Cmd::EnterDirMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Dir(DirCmd::Show { window, path: None })])
            }
            Cmd::EnterManageMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Manage(ManageCmd::Show { window })])
            }
            Cmd::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Cmd::EnterVisualLineMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::VisualLine,
            })]),
            Cmd::EnterSearchMode => Some(vec![FCmd::Search(SearchCmd::Search {
                view,
                doc: s.p.grep.doc,
                bounds: None,
            })]),
            Cmd::EnterGrepMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Grep(GrepCmd::Search { window })])
            }
            Cmd::Quit => common::quit(s, view),
        }
    }
}

impl KeyInputHandler for GrepKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::GrepMode }

    fn keymap(&self) -> crate::types::KeymapHelp { self.keymap.help("grep") }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = s.active_view() else {
            self.pending_keys.clear();
            return None;
        };
        let (_, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)?;

        if view != s.p.grep.view || dse.mode != DocStoreTypes::Mode::Grep {
            self.pending_keys.clear();
            return None;
        }

        self.pending_keys.push(KeyChord::from(event));

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
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
