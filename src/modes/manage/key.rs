use crossterm::event::KeyEvent;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::{
        common,
        manage::{cmd::Cmd, keymap},
        search,
    },
    protocols::{action::ActionCmd, dir::DirCmd, manage::ManageCmd},
    state::{DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct ManageKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl ManageKeyInput {
    pub fn new() -> Self {
        Self { keymap: keymap::keymap(), pending_keys: Vec::new(), last_view: None }
    }

    fn exec(&self, state: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = state.manage.view;
        let doc = state.manage.doc;

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(state, view, motion),
            Cmd::YankLine => common::yank_line(state, view),
            Cmd::ScrollView(motion) => common::scroll_view(state, view, motion),
            Cmd::Jump => common::jump(state, view),
            Cmd::Select => Some(vec![FCmd::Manage(ManageCmd::Select)]),
            Cmd::Close => Some(vec![FCmd::Manage(ManageCmd::Close)]),
            Cmd::FocusWindow(motion) => common::focus_window(state, motion),
            Cmd::EnterDirMode => {
                let Some(window) = state.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Dir(DirCmd::ReplaceWindow { window })])
            }
            Cmd::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Cmd::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Cmd::Quit => common::quit(state, view),
        }
    }
}

impl KeyInputHandler for ManageKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::ManageMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return None;
        };
        let Some((_, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            return None;
        };

        if dse.mode != DocStoreTypes::Mode::Manage {
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
                let cmds = Some(self.exec(state, cmd).unwrap_or_default());
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
