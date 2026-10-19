use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::{
    fed::FCmd,
    modes::common,
    protocols::mp::MpKind,
    state::State,
    types::{MouseInputHandler, MouseInputPriority, render::WindowId},
};

pub struct MpMouseInput {}

impl MouseInputHandler for MpMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::MpMode }

    fn mouse(&mut self, s: &State, window: WindowId, event: MouseEvent) -> Option<Vec<FCmd>> {
        if s.p.mp.window != Some(window) {
            return None;
        }

        assert!(s.p.mp.kind != MpKind::None);

        if s.p.mp.kind == MpKind::Message {
            // Consume the click without focusing the message or moving a cursor
            // below it.
            return Some(Vec::new());
        }

        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Some(Vec::new());
        }

        Some(common::click(s, window, s.p.mp.view, event))
    }
}
