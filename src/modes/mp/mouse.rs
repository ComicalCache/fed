use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    fed::FCmd,
    protocols::{action::ActionCmd, mp::MpKind},
    state::State,
    types::{MouseInputHandler, MouseInputPriority, Pos},
    util::debug_panic,
};

pub struct MpMouseInput {}

impl MouseInputHandler for MpMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::MpMode }

    fn mouse(&mut self, s: &State, event: MouseEvent) -> Option<Vec<FCmd>> {
        let mut pos = (event.column, event.row).into();

        if s.p.mp.window.is_none() || s.p.mp.window != s.workspace.get_window(pos) {
            return None;
        }

        debug_assert!(s.p.mp.kind != MpKind::None);

        if s.p.mp.kind == MpKind::Message {
            // Consume the click but ignore it to avoid tiles under the message
            // to move the cursor bellow the floating message.
            return Some(Vec::new());
        }

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Some(Vec::new());
        }

        let Some(window) = s.p.mp.window else {
            debug_panic!();
            return Some(Vec::new());
        };
        let Some(rect) = s.workspace.get_rect(window) else {
            debug_panic!();
            return Some(Vec::new());
        };
        let view = s.p.mp.view;
        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return Some(Vec::new());
        };

        if dse.read_only {
            debug_panic!();
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
