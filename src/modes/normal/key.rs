use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    input::{KeyInputHandler, priorities::KeyInputPriority},
    protocols::cursor::CursorCommand,
    state::{State, ViewStoreTypes},
    types::Direction,
};

pub struct NormalKeyInput {
    state: State,

    cursor_tx: flume::Sender<CursorCommand>,
    quit_tx: flume::Sender<()>,
}

impl NormalKeyInput {
    pub fn new(
        state: State, cursor_tx: flume::Sender<CursorCommand>, quit_tx: flume::Sender<()>,
    ) -> Self {
        Self { state, cursor_tx, quit_tx }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let view = {
            let workspace = self.state.workspace.read().unwrap();
            let window_view_map = self.state.window_view_map.read().unwrap();

            workspace.active_window.and_then(|window| window_view_map.get(&window).copied())
        };
        let Some(view) = view else {
            return false;
        };

        let is_normal = {
            let view_store = self.state.view_store.read().unwrap();
            if let Some(view) = view_store.get(&view) {
                view.get::<ViewStoreTypes::Mode>().copied() == Some(ViewStoreTypes::Mode::Normal)
            } else {
                false
            }
        };
        if !is_normal {
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
