mod decorations;
mod doc;
mod events;
mod mode;

pub mod types {
    pub use crate::state::doc::{
        decorations::{DecorationId, Decorations},
        doc::Doc,
        events::Event,
        mode::Mode,
    };
}

use std::collections::HashMap;

use crate::newtype::newtype;

newtype!(DocId, usize);

pub type DocStore = HashMap<DocId, DocStoreEntry>;

#[derive(Default)]
pub struct DocStoreEntry {
    pub doc: types::Doc,
    pub mode: types::Mode,

    pub decs: types::Decorations,
}
