use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::mini_buffer::{command::Command, keymap},
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{MiniBufferStoreTypes, StateLock},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct MiniBufferKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
}

impl MiniBufferKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    ) -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), state_lock, action_tx, mini_buffer_tx }
    }

    fn execute(&self, cmd: Command) {
        let state = self.state_lock.read();
        if state.workspace.active_window != state.mini_buffer_store.window {
            return;
        }
        if state.mini_buffer_store.kind != MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let id = state.mini_buffer_store.id;
        let view = state.mini_buffer_store.view;
        drop(state);

        match cmd {
            Command::Input(text) => {
                let _ = self.action_tx.send(ActionCommand::Insert { view, text: text.clone() });
            }
            Command::Move(motion) => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion,
                    move_anchor: true,
                });
            }
            Command::Backspace => {
                let _ = self.action_tx.send(ActionCommand::Backspace { view });
            }
            Command::Delete => {
                let _ = self.action_tx.send(ActionCommand::Delete { view });
            }
            Command::Submit => {
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Submit);
            }
            Command::Close => {
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Close { id });
            }
        }
    }
}

impl KeyInputHandler for MiniBufferKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::MiniBufferMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        if state.workspace.active_window != state.mini_buffer_store.window {
            return false;
        }
        if state.mini_buffer_store.kind != MiniBufferStoreTypes::Kind::Prompt {
            return false;
        }
        drop(state);

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                self.execute(cmd);
                self.pending_keys.clear();

                true
            }
            ParseResult::Prefix => true,
            ParseResult::Invalid => {
                self.pending_keys.clear();

                // If it's a character input, insert it.
                let KeyCode::Char(ch) = event.code else { return false };

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
                    self.execute(Command::Input(format!("<{modifiers}-{ch}>")));
                } else {
                    self.execute(Command::Input(ch.to_string()));
                }

                true
            }
        }
    }
}
