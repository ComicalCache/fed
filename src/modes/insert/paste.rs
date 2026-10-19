use crate::{
    fed::FCmd,
    modes::common,
    state::{State, ViewStoreTypes},
    types::{PasteInputHandler, PasteInputPriority},
};

pub struct InsertPasteInput {}

impl PasteInputHandler for InsertPasteInput {
    fn priority(&self) -> PasteInputPriority { PasteInputPriority::InsertMode }

    fn paste(&mut self, s: &State, text: &str) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        if s.view_store.get(&view).unwrap().mode() != ViewStoreTypes::Mode::Insert {
            return None;
        }

        Some(common::paste(s, view, text))
    }
}
