use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::mp::{command::Command, keymap},
    protocols::{action::ActionCmd, mp::MpCmd},
    state::{MpTypes, State, ViewId},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct MpKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl MpKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), last_view: None }
    }

    fn exec(&mut self, state: &State, cmd: Command) -> Option<Vec<FCmd>> {
        let id = state.mp.id;
        let view = state.mp.view;

        match cmd {
            Command::Input(text) => {
                Some(vec![FCmd::Action(ActionCmd::Insert { view, text: text.clone() })])
            }
            Command::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Command::Backspace => Some(vec![FCmd::Action(ActionCmd::Backspace { view })]),
            Command::Delete => Some(vec![FCmd::Action(ActionCmd::Delete { view })]),
            Command::Submit => Some(vec![FCmd::Mp(MpCmd::Submit)]),
            Command::Close => Some(vec![FCmd::Mp(MpCmd::Close { id })]),
        }
    }
}

impl KeyInputHandler for MpKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::MiniBufferMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        if state.workspace.active_window != state.mp.window {
            return None;
        }
        if state.mp.kind != MpTypes::Kind::Prompt {
            return None;
        }

        let Some(dse) = state.doc_store.get(&state.mp.doc) else {
            debug_panic!();
            return Some(Vec::new());
        };

        if dse.read_only {
            debug_panic!();
            return Some(Vec::new());
        }

        let view = state.mp.view;

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

                if !modifiers.is_empty() {
                    // Always treat entering execute as consuming the key.
                    let cmds = self
                        .exec(state, Command::Input(format!("<{modifiers}-{ch}>")))
                        .unwrap_or_else(|| Vec::new());
                    Some(cmds)
                } else {
                    // Always treat entering execute as consuming the key.
                    let cmds = self
                        .exec(state, Command::Input(ch.to_string()))
                        .unwrap_or_else(|| Vec::new());
                    Some(cmds)
                }
            }
        }
    }
}
