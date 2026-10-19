use crossterm::event::MouseEvent;

use crate::{
    fed::FCmd,
    modes::common,
    state::{DocStoreTypes, State},
    types::{MouseInputHandler, MouseInputPriority, render::WindowId},
};

pub struct DirMouseInput {}

impl MouseInputHandler for DirMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::DirMode }

    fn mouse(&mut self, s: &State, window: WindowId, event: MouseEvent) -> Option<Vec<FCmd>> {
        let view = s.index.window_to_views(window).unwrap()[0];
        let (_, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if dse.mode != DocStoreTypes::Mode::Dir {
            return None;
        }

        Some(common::mouse(s, window, view, event))
    }
}
