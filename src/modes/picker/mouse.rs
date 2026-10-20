use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::{
    fed::FCmd,
    protocols::{action::ActionCmd, picker::PickerCmd},
    state::State,
    types::{Motion, MouseInputHandler, MouseInputPriority, Pos, render::WindowId},
};

pub struct PickerMouseInput {}

impl MouseInputHandler for PickerMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::PickerMode }

    fn mouse(&mut self, s: &State, window: WindowId, event: MouseEvent) -> Option<Vec<FCmd>> {
        if s.p.picker.window.is_none() || s.p.picker.window != s.workspace.active_window {
            return None;
        }

        if window != s.p.picker.window.unwrap() {
            return Some(Vec::new());
        }

        match event.kind {
            MouseEventKind::ScrollUp => {
                return Some(vec![FCmd::Picker(PickerCmd::ChangeSelection(Motion::Up))]);
            }
            MouseEventKind::ScrollDown => {
                return Some(vec![FCmd::Picker(PickerCmd::ChangeSelection(Motion::Down))]);
            }
            MouseEventKind::Down(MouseButton::Left) => {}
            _ => return Some(Vec::new()),
        }

        let (list_vse, list_dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.list_view)
                .unwrap();

        let rect = s.workspace.get_rect(window).unwrap();
        let pos = Pos::from((event.column, event.row)) - rect.pos;

        // Select the list item.
        let list_content = list_vse.layout(list_dse).content;
        if list_content.contains(pos) {
            let row = pos.y - list_content.pos.y + list_vse.scroll.y;
            if s.p.picker.offset + row >= s.p.picker.filtered.len() {
                return Some(Vec::new());
            }

            return Some(vec![FCmd::Action(ActionCmd::MoveCursorToPos {
                view: s.p.picker.list_view,
                pos: Pos::new(list_vse.scroll.x, row),
                move_anchor: true,
            })]);
        }

        let (input_vse, input_dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.input_view)
                .unwrap();

        // Move the input cursor.
        let input_content = input_vse.layout(input_dse).content;
        if input_content.width == 0 || input_content.height == 0 {
            return Some(Vec::new());
        }

        let pos = pos.saturating_sub(input_content.pos);
        let pos = Pos::new(pos.x.min(input_content.width - 1), pos.y.min(input_content.height - 1))
            + *input_vse.scroll;

        Some(vec![FCmd::Action(ActionCmd::MoveCursorToPos {
            view: s.p.picker.input_view,
            pos,
            move_anchor: true,
        })])
    }
}
