use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::action::ActionCommand,
    state::{State, ViewStoreTypes},
    types::Pos,
};

pub struct NormalMouseInput {
    state: State,

    action_tx: UnboundedSender<ActionCommand>,
}

impl NormalMouseInput {
    pub fn new(state: State, action_tx: UnboundedSender<ActionCommand>) -> Self {
        Self { state, action_tx }
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

        if self.state.with_view(view, |vm| vm.mode) != Some(ViewStoreTypes::Mode::Normal) {
            return false;
        }

        let (scroll, layout) =
            self.state.with_view(view, |vm| (vm.scroll, vm.layout)).unwrap_or_default();

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        pos = pos.saturating_sub(rect.pos);

        if pos.x < layout.gutter || pos.y >= rect.height.saturating_sub(layout.mode_line) {
            return false;
        }

        // Offset the physical x by the gutter width to get the actual text column.
        pos = Pos::new(pos.x - layout.gutter, pos.y) + *scroll;

        if event.modifiers.contains(KeyModifiers::ALT) {
            let _ = self.action_tx.send(ActionCommand::CreateCursor { view, pos });
        } else {
            let _ = self.action_tx.send(ActionCommand::MoveCursorTo { view, pos });
        }

        true
    }
}
