use crate::{
    fed::FCmd,
    modes::common,
    state::{State, ViewStoreTypes},
    types::{PasteInputHandler, PasteInputPriority},
};

pub struct NormalPasteInput {}

impl PasteInputHandler for NormalPasteInput {
    fn priority(&self) -> PasteInputPriority { PasteInputPriority::NormalMode }

    fn paste(&mut self, s: &State, text: &str) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        if s.view_store.get(&view).unwrap().mode() != ViewStoreTypes::Mode::Normal {
            return None;
        }

        Some(common::paste(s, view, text))
    }
}
