use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::action::ActionCommand,
    state::{MiniBufferStoreTypes, StateLock},
    types::Pos,
};

pub struct MiniBufferMouseInput {
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
}

impl MiniBufferMouseInput {
    pub fn new(state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>) -> Self {
        Self { state_lock, action_tx }
    }
}

impl MouseInputHandler for MiniBufferMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::MiniBufferMode }

    fn mouse(&mut self, event: &MouseEvent) -> bool {
        let mut pos = (event.column, event.row).into();

        let state = self.state_lock.read();

        if state.mini_buffer_store.window != state.workspace.get_window(pos) {
            return false;
        }

        // If the mini buffer window is not none, neither must the kind be none.
        debug_assert!(state.mini_buffer_store.kind != MiniBufferStoreTypes::Kind::None);

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::Message {
            // Consume the click but ignore it to avoid tiles under the message to move the
            // cursor bellow the floating message.
            return true;
        }

        let Some(window) = state.mini_buffer_store.window else { return false };
        let Some(rect) = state.workspace.get_rect(window) else { return false };
        let view = state.mini_buffer_store.view;
        let Some((vse, dse)) = state.view_and_doc(view) else { return false };

        let lines = dse.doc.data.lines();
        let scroll = vse.scroll;
        let layout = vse.layout;

        drop(state);

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        pos = pos.saturating_sub(rect.pos);

        if pos.x < layout.gutter_width(lines)
            || pos.y >= rect.height.saturating_sub(layout.mode_line)
        {
            return false;
        }

        // Offset the physical x by the gutter width to get the actual text column.
        pos = Pos::new(pos.x - layout.gutter_width(lines), pos.y) + *scroll;

        if event.modifiers.contains(KeyModifiers::ALT) {
            let _ = self.action_tx.send(ActionCommand::CreateCursor { view, pos });
        } else {
            let _ = self.action_tx.send(ActionCommand::MoveCursorTo { view, pos });
        }

        true
    }
}
