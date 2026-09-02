use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::cursor::CursorCommand,
    state::{State, ViewStoreTypes},
    types::Pos,
};

pub struct NormalMouseInput {
    state: State,

    cursor_tx: UnboundedSender<CursorCommand>,
}

impl NormalMouseInput {
    pub fn new(state: State, cursor_tx: UnboundedSender<CursorCommand>) -> Self {
        Self { state, cursor_tx }
    }
}

impl MouseInputHandler for NormalMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::NormalMode }

    fn mouse(&mut self, event: &MouseEvent) -> bool {
        let mut pos = (event.column, event.row).into();

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

        let (scroll, layout) = {
            let view_store = self.state.view_store.read().unwrap();
            if let Some(view) = view_store.get(&view) {
                let scroll =
                    view.get::<ViewStoreTypes::Scroll>().map(|&scroll| scroll).unwrap_or_default();
                let layout = view.get::<ViewStoreTypes::Layout>().copied().unwrap_or_default();

                (scroll, layout)
            } else {
                (ViewStoreTypes::Scroll(Pos::default()), ViewStoreTypes::Layout::default())
            }
        };

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        pos = pos.saturating_sub(rect.pos);

        if pos.x < layout.gutter || pos.y >= rect.height.saturating_sub(layout.mode_line) {
            return false;
        }

        // Offset the physical x by the gutter width to get the actual text column!
        pos = Pos::new(pos.x - layout.gutter, pos.y) + *scroll;

        let _ = self.cursor_tx.send(CursorCommand::MoveTo { view, pos });

        true
    }
}
