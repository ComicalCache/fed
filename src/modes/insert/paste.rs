use crate::fed::FCmd;
use crate::modes::common;
use crate::state::State;
use crate::state::ViewStoreTypes;
use crate::types::PasteInputHandler;
use crate::types::PasteInputPriority;

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
