use crossterm::event::MouseEvent;

use crate::{
    fed::FCmd,
    modes::common,
    state::{State, ViewStoreTypes},
    types::{MouseInputHandler, MouseInputPriority, render::WindowId},
};

pub struct SearchMouseInput {}

impl MouseInputHandler for SearchMouseInput {
    fn priority(&self) -> MouseInputPriority { MouseInputPriority::SearchMode }

    fn mouse(&mut self, s: &State, window: WindowId, event: MouseEvent) -> Option<Vec<FCmd>> {
        let view = s.index.window_to_views(window).unwrap()[0];
        let (vse, _) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if vse.mode() != ViewStoreTypes::Mode::Search {
            return None;
        }

        Some(common::click(s, window, view, event))
    }
}
