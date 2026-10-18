use crossterm::event::KeyEvent;

use crate::{
    fed::FCmd,
    modes::{
        common,
        read::{cmd::Cmd, keymap},
        search,
    },
    protocols::{action::ActionCmd, dir::DirCmd, manage::ManageCmd},
    state::{DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, ParseResult},
    util::debug_panic,
};

pub struct ReadKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl ReadKeyInput {
    pub fn new() -> Self {
        Self { keymap: keymap::keymap(), pending_keys: Vec::new(), last_view: None }
    }

    fn exec(s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let Some(doc) = s.index.view_to_doc(view) else {
            debug_panic!();
            return None;
        };

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(s, view, motion),
            Cmd::YankLine => common::yank_line(s, view),
            Cmd::ScrollView(motion) => common::scroll_view(s, view, motion),
            Cmd::Jump => common::jump(s, view),
            Cmd::SplitWindow(direction) => common::split_window(s, doc, direction),
            Cmd::FocusWindow(motion) => common::focus_window(s, motion),
            Cmd::EnterManageMode => {
                let Some(window) = s.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Manage(ManageCmd::ReplaceWindow { window })])
            }
            Cmd::EnterDirMode => {
                let Some(window) = s.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Dir(DirCmd::ReplaceWindow { window, path: None })])
            }

            Cmd::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Cmd::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Cmd::Quit => common::quit(s, view),
        }
    }
}

impl KeyInputHandler for ReadKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::ReadMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let Some((_, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view) else {
            debug_panic!();
            return None;
        };

        if dse.mode != DocStoreTypes::Mode::Read {
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
