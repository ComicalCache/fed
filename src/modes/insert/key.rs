use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::action::ActionCommand,
    state::{State, ViewStoreTypes},
    types::Direction,
};

pub struct InsertKeyInput {
    state: State,

    action_tx: UnboundedSender<ActionCommand>,
}

impl InsertKeyInput {
    pub fn new(state: State, action_tx: UnboundedSender<ActionCommand>) -> Self {
        Self { state, action_tx }
    }
}

impl KeyInputHandler for InsertKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::InsertMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let Some(view) = self
            .state
            .with_workspace(|w| w.active_window)
            .and_then(|window| self.state.with_window_view_map(|wv| wv.get(&window).cloned())?)
        else {
            return false;
        };

        if self.state.with_view(view, |vm| vm.mode) != Some(ViewStoreTypes::Mode::Insert) {
            return false;
        }

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
                self.state.with_view(view, |view| view.doc).and_then(|doc| {
                    self.state.with_doc_mut(doc, |d| {
                        d.doc.data.end_commit();
                    })
                });

                self.state.with_view_mut(view, |vm| vm.mode = ViewStoreTypes::Mode::Normal);

                return true;
            }
            _ => return false,
        }

        true
    }
}
