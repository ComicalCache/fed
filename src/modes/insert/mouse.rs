use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    fed::FCmd,
    protocols::action::ActionCmd,
    state::{State, ViewStoreTypes},
    types::{MouseInputHandler, MouseInputPriority},
    util::debug_panic,
};

pub struct InsertMouseInput {}

impl MouseInputHandler for InsertMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::InsertMode }

    fn mouse(&mut self, s: &State, event: MouseEvent) -> Option<Vec<FCmd>> {
        let mut pos = (event.column, event.row).into();

        // Click outside of any window, just abort.
        let Some(window) = s.workspace.get_window(pos) else { return None };
        let Some(rect) = s.workspace.get_rect(window) else {
            debug_panic!();
            return None;
        };
        if !rect.contains(pos) {
            return None;
        }
        let Some(view) = s.active_view() else {
            debug_panic!();
            return None;
        };
        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
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

        let content = vse.layout(dse).content;
        if !content.contains(pos) {
            return Some(Vec::new());
        }

        pos = pos.saturating_sub(content.pos) + *vse.scroll;

        if event.modifiers.contains(KeyModifiers::ALT) {
            Some(vec![FCmd::Action(ActionCmd::CreateCursorAtPos { view, pos })])
        } else {
            Some(vec![FCmd::Action(ActionCmd::MoveCursorToPos { view, pos, move_anchor: true })])
        }
    }
}
