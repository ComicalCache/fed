use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::action::ActionCommand,
    state::{StateLock, ViewStoreTypes},
    types::Direction,
};

pub struct NormalKeyInput {
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    quit_tx: UnboundedSender<()>,
}

impl NormalKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        quit_tx: UnboundedSender<()>,
    ) -> Self {
        Self { state_lock, action_tx, quit_tx }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();

        let Some(view) = state.active_view() else { return false };
        let Some(doc) = state.view_store.get(&view).map(|vse| vse.doc) else { return false };

        if state.view_store.get(&view).map(|vse| vse.mode) != Some(ViewStoreTypes::Mode::Normal) {
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
            _ => return false,
        };

        true
    }
}
