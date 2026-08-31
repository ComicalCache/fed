use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::{
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::cursor::CursorCommand,
    state::{State, ViewStoreTypes},
    types::Pos,
};

pub struct NormalMouseInput {
    state: State,

    cursor_tx: flume::Sender<CursorCommand>,
}

impl NormalMouseInput {
    pub fn new(state: State, cursor_tx: flume::Sender<CursorCommand>) -> Self {
        Self { state, cursor_tx }
    }
}

impl MouseInputHandler for NormalMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::NormalMode }

    fn mouse(&mut self, event: &MouseEvent) -> bool {
        let pos = (event.column, event.row).into();

        let (window, rect) = {
            let mut workspace = self.state.workspace.write().unwrap();

            let Some(window) = workspace.get_window(pos) else {
                return false;
            };

            workspace.active_window = Some(window);

            (window, workspace.get_rect(window).unwrap())
        };

        let view = {
            let window_view_map = self.state.window_view_map.read().unwrap();
            window_view_map.get(&window).copied()
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

        let scroll = {
            let view_store = self.state.view_store.read().unwrap();
            if let Some(view) = view_store.get(&view) {
                view.get::<ViewStoreTypes::Scroll>().map(|&scroll| scroll).unwrap_or_default()
            } else {
                ViewStoreTypes::Scroll(Pos::default())
            }
        };

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        let _ = self
            .cursor_tx
            .send(CursorCommand::MoveTo { view, pos: pos.saturating_sub(rect.pos) + *scroll });

        true
    }
}
