mod decorations;
mod document;
mod mode;

pub mod types {
    pub use crate::state::document::{decorations::Decorations, document::Document, mode::Mode};
}

use std::collections::HashMap;

use crate::newtype::newtype;

newtype!(DocumentId, u64);

pub type DocumentStore = HashMap<DocumentId, DocumentStoreEntry>;

#[derive(Default)]
pub struct DocumentStoreEntry {
    pub doc: types::Document,
    pub mode: types::Mode,
    pub decs: types::Decorations,
}
