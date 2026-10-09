use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{MouseInputHandler, MouseInputPriority},
    protocols::action::ActionCmd,
    state::{DocStoreTypes, State},
    types::Pos,
};

pub struct DirMouseInput {}

impl MouseInputHandler for DirMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::DirMode }

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

        if dse.mode != DocStoreTypes::Mode::Dir {
            return None;
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

        Some(vec![FCmd::Action(ActionCmd::MoveCursorToPos { view, pos, move_anchor: true })])
    }
}
