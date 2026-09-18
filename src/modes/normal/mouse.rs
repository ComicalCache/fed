use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::action::ActionCommand,
    state::{StateLock, ViewStoreTypes},
    types::Pos,
};

pub struct NormalMouseInput {
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
}

impl NormalMouseInput {
    pub fn new(state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>) -> Self {
        Self { state_lock, action_tx }
    }
}

impl MouseInputHandler for NormalMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::NormalMode }

    fn mouse(&mut self, event: &MouseEvent) -> bool {
        let mut pos = (event.column, event.row).into();

        let state = self.state_lock.read();

        let Some(window) = state.workspace.get_window(pos) else { return false };
        let Some(rect) = state.workspace.get_rect(window) else { return false };
        let Some(view) = state.active_view() else { return false };
        let Some((vse, dse)) = state.vse_and_dse(view) else { return false };

        if vse.mode != ViewStoreTypes::Mode::Normal {
            return false;
        }

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        let lines = dse.doc.data.lines();
        let scroll = vse.scroll;
        let layout = vse.layout;

        drop(state);

        pos = pos.saturating_sub(rect.pos);

        if pos.x < layout.gutter_width(lines)
            || pos.y >= rect.height.saturating_sub(layout.mode_line)
        {
            return false;
        }

        // Offset the physical x by the gutter width to get the actual text column.
        pos = Pos::new(pos.x - layout.gutter_width(lines), pos.y) + *scroll;

        if event.modifiers.contains(KeyModifiers::ALT) {
            let _ = self.action_tx.send(ActionCommand::CreateCursorAtPos { view, pos });
        } else {
            let _ = self.action_tx.send(ActionCommand::MoveCursorToPos { view, pos });
        }

        true
    }
}
