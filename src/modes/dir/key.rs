use std::path::PathBuf;

use crossterm::event::KeyEvent;

use crate::{
    fed::FCmd,
    modes::{
        common,
        dir::{cmd::Cmd, keymap},
        search,
    },
    protocols::{
        action::ActionCmd,
        dir::{DirCmd, DirEntry, DirProtocol},
        manage::ManageCmd,
        mp::MpCmd,
    },
    state::{DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, ParseResult},
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

    fn exec(s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.p.dir.view;
        let doc = s.p.dir.doc;

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(s, view, motion),
            Cmd::YankLine => common::yank_line(s, view),
            Cmd::ScrollView(motion) => Some(common::scroll_view(s, view, motion)),
            Cmd::Jump => common::jump(s, view),
            Cmd::Create => Some(Self::create(s)),
            Cmd::Rename => Self::rename(s, view),
            Cmd::Delete => Self::delete(s, view, false),
            Cmd::DeleteRecursive => Self::delete(s, view, true),
            Cmd::Select => Some(vec![FCmd::Dir(DirCmd::Select)]),
            Cmd::FocusWindow(motion) => common::focus_window(s, motion),
            Cmd::EnterManageMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Manage(ManageCmd::ReplaceWindow { window })])
            }
            Cmd::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Cmd::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Cmd::Quit => common::quit(s, view),
        }
    }

    fn create(s: &State) -> Vec<FCmd> {
        let mut initial_text = s.p.dir.pwd.display().to_string();

        if !initial_text.ends_with(std::path::MAIN_SEPARATOR) {
            initial_text.push(std::path::MAIN_SEPARATOR);
        }

        let pcallback = Box::new(move |s: &mut State, path: String| {
            let dir = path.ends_with(std::path::MAIN_SEPARATOR);
            let path = PathBuf::from(path);

            if !path.is_empty() {
                DirProtocol::create(s, path, dir);
            }
        });

        vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "New entry (trailing '/' for directory): ".to_string(),
            initial_text: Some(initial_text),
            pcallback,
            tx: None,
        })]
    }

    fn rename(s: &State, view: ViewId) -> Option<Vec<FCmd>> {
        let entry = Self::curr_entry(s, view)?;
        entry.path.file_name()?;

        let old = entry.path.clone();
        let pcallback = Box::new(move |s: &mut State, new: String| {
            let new = util::path::normalize(new);

            if !new.is_empty() {
                DirProtocol::rename(s, old, new);
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "New path: ".to_string(),
            initial_text: Some(entry.path.display().to_string()),
            pcallback,
            tx: None,
        })])
    }

    fn delete(s: &State, view: ViewId, recursive: bool) -> Option<Vec<FCmd>> {
        let entry = Self::curr_entry(s, view)?;
        entry.path.file_name()?;

        let path = entry.path;
        let name = path.display().to_string();

        let prompt = if recursive {
            format!("Delete recursively '{name}'? (y/N): ")
        } else {
            format!("Delete '{name}'? (y/N): ")
        };

        let pcallback = Box::new(move |s: &mut State, res: String| {
            if res.to_lowercase() == "y" {
                DirProtocol::delete(s, path, recursive);
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt { prompt, initial_text: None, pcallback, tx: None })])
    }

    fn curr_entry(s: &State, view: ViewId) -> Option<DirEntry> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let y = dse.doc.data.get_line_of_byte(offset);

        // Ignore the header and ".." entry.
        if y < 2 {
            return None;
        }

        s.p.dir.entries.get(y).cloned()
    }
}

impl KeyInputHandler for DirKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::DirMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let (_, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if view != s.p.dir.view || dse.mode != DocStoreTypes::Mode::Dir {
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
