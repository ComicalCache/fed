use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::action::ActionCommand,
    state::{StateLock, ViewStoreTypes},
    types::Pos,
};

pub struct InsertMouseInput {
    state_lock: StateLock,

    cursor_tx: UnboundedSender<ActionCommand>,
}

impl InsertMouseInput {
    pub fn new(state_lock: StateLock, cursor_tx: UnboundedSender<ActionCommand>) -> Self {
        Self { state_lock, cursor_tx }
    }
}

impl MouseInputHandler for InsertMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::InsertMode }

    fn mouse(&mut self, event: &MouseEvent) -> bool {
        let mut pos = (event.column, event.row).into();

        let mut state = self.state_lock.write();

        let Some(window) = state.workspace.get_window(pos) else { return false };

        state.workspace.active_window = Some(window);

        let Some(rect) = state.workspace.get_rect(window) else { return false };
        let Some(view) = state.active_view() else { return false };
        let Some(vse) = state.view_store.get(&view) else { return false };
        let scroll = vse.scroll;
        let layout = vse.layout;

        if vse.mode != ViewStoreTypes::Mode::Insert {
            return false;
        }

        drop(state);

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
            let _ = self.cursor_tx.send(ActionCommand::CreateCursor { view, pos });
        } else {
            let _ = self.cursor_tx.send(ActionCommand::MoveCursorTo { view, pos });
        }

        true
    }
}
