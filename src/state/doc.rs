mod decorations;
mod doc;
mod event;
mod mode;

pub mod types {
    pub use crate::state::doc::decorations::DecorationId;
    pub use crate::state::doc::decorations::Decorations;
    pub use crate::state::doc::doc::Doc;
    pub use crate::state::doc::event::Event;
    pub use crate::state::doc::mode::Mode;
}

use std::collections::HashMap;

use crate::util::newtype;

newtype!(DocId, usize, Debug, Default, Clone, Copy, PartialEq, Eq, Hash);

pub type DocStore = HashMap<DocId, DocStoreEntry>;

#[derive(Default)]
pub struct DocStoreEntry {
    pub doc: types::Doc,
    pub mode: types::Mode,

    pub read_only: bool,
    pub max_cursors: Option<usize>,

    pub decs: types::Decorations,
}
