mod decorations;
mod document;
mod events;
mod mode;

pub mod types {
    pub use crate::state::document::{
        decorations::{Decorations, DocumentDecoration},
        document::Document,
        events::DocumentEvent,
        mode::Mode,
    };
}

use std::collections::HashMap;

use crate::newtype::newtype;

newtype!(DocumentId, usize);

pub type DocumentStore = HashMap<DocumentId, DocumentStoreEntry>;

#[derive(Default)]
pub struct DocumentStoreEntry {
    pub doc: types::Document,
    pub mode: types::Mode,

    pub decs: types::Decorations,
}
