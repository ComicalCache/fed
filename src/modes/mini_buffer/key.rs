use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{MiniBufferStoreTypes, StateLock},
    types::Direction,
};

pub struct MiniBufferKeyInput {
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
}

impl MiniBufferKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    ) -> Self {
        Self { state_lock, action_tx, mini_buffer_tx }
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

        let id = state.mini_buffer_store.id;
        let view = state.mini_buffer_store.view;
        drop(state);

        match event.code {
            KeyCode::Left => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Left });
            }
            KeyCode::Right => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Right });
            }
            KeyCode::Up => {}   // TODO: Prompt history previous/next auto complete?
            KeyCode::Down => {} // TODO: Prompt history next/previous auto complete?
            KeyCode::Backspace => {
                let _ = self.action_tx.send(ActionCommand::Backspace { view });
            }
            KeyCode::Delete => {
                let _ = self.action_tx.send(ActionCommand::Delete { view });
            }
            KeyCode::Char(ch) => {
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
                    let _ = self
                        .action_tx
                        .send(ActionCommand::Insert { view, text: format!("<{modifiers}-{ch}>") });
                } else {
                    let _ =
                        self.action_tx.send(ActionCommand::Insert { view, text: ch.to_string() });
                }
            }
            KeyCode::Enter => {
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Submit);
            }
            KeyCode::Tab => {
                let _ = self.action_tx.send(ActionCommand::Insert { view, text: "\t".to_string() });
            }
            KeyCode::Esc => {
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Close { id });
            }
            _ => return false,
        }

        true
    }
}
