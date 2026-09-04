use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::cursor::CursorCommand,
    state::{State, ViewStoreTypes},
    types::Direction,
};

pub struct NormalKeyInput {
    state: State,

    cursor_tx: UnboundedSender<CursorCommand>,
    quit_tx: UnboundedSender<()>,
}

impl NormalKeyInput {
    pub fn new(
        state: State, cursor_tx: UnboundedSender<CursorCommand>, quit_tx: UnboundedSender<()>,
    ) -> Self {
        Self { state, cursor_tx, quit_tx }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let Some(view) = self
            .state
            .with_workspace(|w| w.active_window)
            .and_then(|window| self.state.with_window_view_map(|wv| wv.get(&window).cloned())?)
        else {
            return false;
        };

        if self.state.with_view(view, |vm| vm.get::<ViewStoreTypes::Mode>().cloned()).flatten()
            != Some(ViewStoreTypes::Mode::Normal)
        {
            return false;
        }

        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('q') {
            let _ = self.quit_tx.send(());

            return true;
        }

        let direction = match event.code {
            KeyCode::Char('h') => Direction::Left,
            KeyCode::Char('j') => Direction::Down,
            KeyCode::Char('k') => Direction::Up,
            KeyCode::Char('l') => Direction::Right,
            _ => return false,
        };

        let _ = self.cursor_tx.send(CursorCommand::Move { view, direction });

        true
    }
}
