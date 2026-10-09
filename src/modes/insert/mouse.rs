use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{MouseInputHandler, MouseInputPriority},
    protocols::action::ActionCmd,
    state::{State, ViewStoreTypes},
    types::Pos,
};

pub struct InsertMouseInput {}

impl MouseInputHandler for InsertMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::InsertMode }

    fn mouse(&mut self, state: &State, event: &MouseEvent) -> Option<Vec<FCmd>> {
        let mut pos = (event.column, event.row).into();

        let Some(window) = state.workspace.get_window(pos) else {
            // Click outside of any window, just abort.
            return None;
        };
        let Some(rect) = state.workspace.get_rect(window) else {
            debug_panic!();
            return None;
        };
        let Some(view) = state.active_view() else {
            debug_panic!();
            return None;
        };
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        if vse.mode() != ViewStoreTypes::Mode::Insert {
            return None;
        }
        if dse.read_only {
            debug_panic!();
            return Some(Vec::new());
        }

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Some(Vec::new());
        }

        pos = pos.saturating_sub(rect.pos);

        let gutter_width = vse.layout.gutter_width(dse.doc.data.lines());
        if pos.x < gutter_width || pos.y >= rect.height.saturating_sub(vse.layout.mode_line) {
            return Some(Vec::new());
        }

        // Offset the physical x by the gutter width to get the actual text
        // column.
        pos = Pos::new(pos.x.saturating_sub(gutter_width), pos.y) + *vse.scroll;

        if event.modifiers.contains(KeyModifiers::ALT) {
            Some(vec![FCmd::Action(ActionCmd::CreateCursorAtPos { view, pos })])
        } else {
            Some(vec![FCmd::Action(ActionCmd::MoveCursorToPos { view, pos, move_anchor: true })])
        }
    }
}
