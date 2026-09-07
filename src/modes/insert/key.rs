use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::action::ActionCommand,
    state::{StateLock, ViewStoreTypes},
    types::Direction,
};

pub struct InsertKeyInput {
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
}

impl InsertKeyInput {
    pub fn new(state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>) -> Self {
        Self { state_lock, action_tx }
    }
}

impl KeyInputHandler for InsertKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::InsertMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();

        let Some(view) = state.active_view() else { return false };
        let Some(doc) = state.view_store.get(&view).map(|vse| vse.doc) else { return false };

        if state.view_store.get(&view).map(|vse| vse.mode) != Some(ViewStoreTypes::Mode::Insert) {
            return false;
        }

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
            KeyCode::Up => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Up });
            }
            KeyCode::Down => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Down });
            }
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
                    let _ = self.action_tx.send(ActionCommand::InsertText {
                        view,
                        text: format!("<{modifiers}-{ch}>"),
                    });
                } else {
                    let _ = self
                        .action_tx
                        .send(ActionCommand::InsertText { view, text: ch.to_string() });
                }
            }
            KeyCode::Enter => {
                let _ = self.action_tx.send(ActionCommand::InsertNewline { view });
            }
            KeyCode::Tab => {
                let _ = self.action_tx.send(ActionCommand::InsertTab { view });
            }
            KeyCode::Esc => {
                let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Normal });

                return true;
            }
            _ => return false,
        }

        true
    }
}
