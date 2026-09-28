use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    debug_panic::debug_panic,
    input::{MouseInputHandler, priorities::MouseInputPriority},
    protocols::action::ActionCommand,
    state::{MiniBufferStoreTypes, State, StateLock},
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
        if state.mini_buffer_store.window.is_none()
            || state.mini_buffer_store.window != state.workspace.get_window(pos)
        {
            return false;
        }

        debug_assert!(state.mini_buffer_store.kind != MiniBufferStoreTypes::Kind::None);

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::Message {
            // Consume the click but ignore it to avoid tiles under the message
            // to move the cursor bellow the floating message.
            return true;
        }

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return true;
        }

        let Some(window) = state.mini_buffer_store.window else {
            debug_panic!();
            return true;
        };
        let Some(rect) = state.workspace.get_rect(window) else {
            debug_panic!();
            return true;
        };
        let view = state.mini_buffer_store.view;
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return true;
        };

        let scroll = vse.scroll;
        let mode_line = vse.layout.mode_line;
        let gutter_width = vse.layout.gutter_width(dse.doc.data.lines());
        drop(state);

        pos = pos.saturating_sub(rect.pos);

        if pos.x < gutter_width || pos.y >= rect.height.saturating_sub(mode_line) {
            return true;
        }

        // Offset the physical x by the gutter width to get the actual text
        // column.
        pos = Pos::new(pos.x.saturating_sub(gutter_width), pos.y) + *scroll;

        if event.modifiers.contains(KeyModifiers::ALT) {
            let _ = self.action_tx.send(ActionCommand::CreateCursorAtPos { view, pos });
        } else {
            let _ = self.action_tx.send(ActionCommand::MoveCursorToPos {
                view,
                pos,
                move_anchor: true,
            });
        }

        true
    }
}
