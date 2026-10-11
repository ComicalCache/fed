use std::path::PathBuf;

use crossterm::event::KeyEvent;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::{
        common,
        dir::{cmd::Cmd, keymap},
        search,
    },
    protocols::{
        action::ActionCmd,
        dir::{DirCmd, DirProtocol},
        manage::ManageCmd,
        mp::MpCmd,
        state::PState,
    },
    state::{DirTypes, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, ParseResult},
    util,
};

pub struct DirKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl DirKeyInput {
    pub fn new() -> Self {
        Self { keymap: keymap::keymap(), pending_keys: Vec::new(), last_view: None }
    }

    fn exec(&mut self, state: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = state.dir.view;
        let doc = state.dir.doc;

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(state, view, motion),
            Cmd::YankLine => common::yank_line(state, view),
            Cmd::ScrollView(motion) => common::scroll_view(state, view, motion),
            Cmd::Jump => common::jump(state, view),
            Cmd::Create => self.create(state),
            Cmd::Rename => self.rename(state, view),
            Cmd::Delete => self.delete(state, view, false),
            Cmd::DeleteRecursive => self.delete(state, view, true),
            Cmd::Select => Some(vec![FCmd::Dir(DirCmd::Select)]),
            Cmd::FocusWindow(motion) => common::focus_window(state, motion),
            Cmd::EnterManageMode => {
                let Some(window) = state.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Manage(ManageCmd::ReplaceWindow { window })])
            }
            Cmd::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Cmd::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Cmd::Quit => common::quit(state, view),
        }
    }

    fn create(&self, state: &State) -> Option<Vec<FCmd>> {
        let mut initial_text = state.dir.pwd.display().to_string();

        if !initial_text.ends_with(std::path::MAIN_SEPARATOR) {
            initial_text.push(std::path::MAIN_SEPARATOR);
        }

        let pcallback = Box::new(move |state: &mut State, _: &mut PState, path: String| {
            let dir = path.ends_with(std::path::MAIN_SEPARATOR);
            let path = PathBuf::from(path);

            if !path.is_empty() {
                DirProtocol::create(state, path, dir);
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "New entry (trailing '/' for directory): ".to_string(),
            initial_text: Some(initial_text),
            pcallback,
            tx: None,
        })])
    }

    fn rename(&self, state: &State, view: ViewId) -> Option<Vec<FCmd>> {
        let Some(entry) = self.curr_entry(state, view) else { return None };
        if entry.path.file_name().is_none() {
            return None;
        }

        let old = entry.path.clone();
        let pcallback = Box::new(move |state: &mut State, _: &mut PState, new: String| {
            let new = util::path::normalize(new);

            if !new.is_empty() {
                DirProtocol::rename(state, old, new);
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "New path: ".to_string(),
            initial_text: Some(entry.path.display().to_string()),
            pcallback,
            tx: None,
        })])
    }

    fn delete(&self, state: &State, view: ViewId, recursive: bool) -> Option<Vec<FCmd>> {
        let Some(entry) = self.curr_entry(state, view) else { return None };
        if entry.path.file_name().is_none() {
            return None;
        }

        let path = entry.path.clone();
        let name = path.display().to_string();

        let prompt = if recursive {
            format!("Delete recursively '{}'? (y/N): ", name)
        } else {
            format!("Delete '{}'? (y/N): ", name)
        };

        let pcallback = Box::new(move |state: &mut State, _: &mut PState, res: String| {
            if res.to_lowercase() == "y" {
                DirProtocol::delete(state, path, recursive);
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt { prompt, initial_text: None, pcallback, tx: None })])
    }

    fn curr_entry(&self, state: &State, view: ViewId) -> Option<DirTypes::Entry> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let y = dse.doc.data.get_line_of_byte(offset);

        // Ignore the header and ".." entry.
        if y < 2 {
            return None;
        }

        state.dir.entries.get(y).cloned()
    }
}

impl KeyInputHandler for DirKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::DirMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return None;
        };
        let Some((_, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        if dse.mode != DocStoreTypes::Mode::Dir {
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
