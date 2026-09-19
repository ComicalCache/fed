use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{StateLock, ViewStoreTypes},
    types::Direction,
};

pub struct NormalKeyInput {
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    quit_tx: UnboundedSender<()>,
}

impl NormalKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>, quit_tx: UnboundedSender<()>,
    ) -> Self {
        Self { state_lock, action_tx, mini_buffer_tx, quit_tx }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return false;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return false;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return false;
        };

        if vse.mode != ViewStoreTypes::Mode::Normal {
            return false;
        }
        drop(state);

        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('q') {
            let _ = self.quit_tx.send(());

            return true;
        }

        match event.code {
            KeyCode::Char('h') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Left });
            }
            KeyCode::Char('j') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Down });
            }
            KeyCode::Char('k') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Up });
            }
            KeyCode::Char('l') => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::MoveCursors { view, direction: Direction::Right });
            }
            KeyCode::Char('i') => {
                let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
            }
            KeyCode::Char('T') => {
                let (tx, rx) = oneshot::channel();
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Message {
                    message: "Hello, world!".to_string(),
                    tx: tx,
                });

                let tx = self.mini_buffer_tx.clone();
                tokio::spawn(async move {
                    let Ok(id) = rx.await else { return };
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = tx.send(MiniBufferCommand::Close { id });
                });
            }
            KeyCode::Char('P') => {
                let (id_tx, _) = oneshot::channel();
                let (res_tx, res_rx) = oneshot::channel();
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Prompt {
                    prompt: "Hello, prompt: ".to_string(),
                    id_tx,
                    res_tx,
                });

                let mini_buffer_tx = self.mini_buffer_tx.clone();
                tokio::spawn(async move {
                    let Ok(res) = res_rx.await else { return };

                    let (tx, rx) = oneshot::channel();
                    let _ = mini_buffer_tx.send(MiniBufferCommand::Message {
                        message: format!("You typed: '{res}'"),
                        tx: tx,
                    });

                    let tx = mini_buffer_tx.clone();
                    tokio::spawn(async move {
                        let Ok(id) = rx.await else { return };
                        tokio::time::sleep(Duration::from_secs(3)).await;

                        let _ = tx.send(MiniBufferCommand::Close { id });
                    });
                });
            }
            _ => return false,
        };

        true
    }
}
