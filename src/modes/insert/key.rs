use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    fed::FCmd,
    modes::insert::{cmd::Cmd, keymap},
    protocols::action::ActionCmd,
    state::{State, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, ParseResult},
    util::debug_panic,
};

pub struct InsertKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    whitespace: bool,

    last_view: Option<ViewId>,
}

impl InsertKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), whitespace: false, last_view: None }
    }

    fn exec(&mut self, s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let Some(doc) = s.index.view_to_doc(view) else {
            debug_panic!();
            return None;
        };

        match cmd {
            Cmd::Input(text) => {
                let mut cmds = Vec::new();

                let whitespace = text.chars().all(char::is_whitespace);
                if whitespace && !self.whitespace {
                    cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
                    cmds.push(FCmd::Action(ActionCmd::StartCommit { doc }));
                }
                self.whitespace = whitespace;

                cmds.push(FCmd::Action(ActionCmd::Insert { view, text }));

                Some(cmds)
            }
            Cmd::Move(motion) => {
                self.whitespace = false;

                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Backspace => {
                self.whitespace = false;

                Some(vec![FCmd::Action(ActionCmd::Backspace { view })])
            }
            Cmd::Delete => {
                self.whitespace = false;

                Some(vec![FCmd::Action(ActionCmd::Delete { view })])
            }
            Cmd::Escape => {
                self.whitespace = false;

                Some(vec![
                    FCmd::Action(ActionCmd::EndCommit { doc }),
                    FCmd::Action(ActionCmd::PopViewMode { view }),
                ])
            }
        }
    }
}

impl KeyInputHandler for InsertKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::InsertMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return None;
        };

        if vse.mode() != ViewStoreTypes::Mode::Insert {
            return None;
        }
        if dse.read_only {
            debug_panic!();
            return Some(Vec::new());
        }

        if self.last_view != Some(view) {
            self.whitespace = false;
            self.pending_keys.clear();

            self.last_view = Some(view);
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                // Always treat entering execute as consuming the key.
                let cmds = Some(self.exec(s, cmd).unwrap_or_default());
                self.pending_keys.clear();

                cmds
            }
            ParseResult::Prefix => Some(Vec::new()),
            ParseResult::Invalid => {
                self.pending_keys.clear();

                // If it's a character input, insert it.
                let KeyCode::Char(ch) = event.code else { return Some(Vec::new()) };

                let modifiers = event
                    .modifiers
                    .iter_names()
                    .filter_map(|(_, m)| match m {
                        KeyModifiers::ALT => Some("A"),
                        KeyModifiers::CONTROL => Some("C"),
                        KeyModifiers::META => Some("M"),
                        KeyModifiers::SUPER => Some("S"),
                        KeyModifiers::HYPER => Some("H"),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("-");

                if modifiers.is_empty() {
                    // Always treat entering execute as consuming the key.
                    let cmds = self.exec(s, Cmd::Input(ch.to_string())).unwrap_or_default();
                    Some(cmds)
                } else {
                    // Always treat entering execute as consuming the key.
                    let cmds =
                        self.exec(s, Cmd::Input(format!("<{modifiers}-{ch}>"))).unwrap_or_default();
                    Some(cmds)
                }
            }
        }
    }
}
