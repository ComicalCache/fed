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

        let Some((view, rect)) =
            self.state.with_workspace(|w| w.get_window(pos)).and_then(|window| {
                self.state.with_workspace_mut(|w| w.active_window = Some(window));

                let view = self.state.with_window_view_map(|wv| wv.get(&window).cloned())??;
                let rect = self.state.with_workspace(|w| w.get_rect(window).unwrap());
                Some((view, rect))
            })
        else {
            return false;
        };

        if self.state.with_view(view, |vm| vm.get::<ViewStoreTypes::Mode>().cloned()).flatten()
            != Some(ViewStoreTypes::Mode::Normal)
        {
            return false;
        }

        let (scroll, layout) = self
            .state
            .with_view(view, |vm| {
                let scroll = vm.get::<ViewStoreTypes::Scroll>().map(|&s| s).unwrap_or_default();
                let layout = vm.get::<ViewStoreTypes::Layout>().cloned().unwrap_or_default();

                (scroll, layout)
            })
            .unwrap_or_else(|| {
                (ViewStoreTypes::Scroll(Pos::default()), ViewStoreTypes::Layout::default())
            });

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
