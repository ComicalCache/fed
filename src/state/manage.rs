mod entry;
mod mode_line;

pub mod types {
    pub use crate::state::manage::{entry::Entry, mode_line::ManageModeLine};
}

use crate::state::{DocId, ViewId};

#[derive(Default)]
pub struct Manage {
    pub doc: DocId,
    pub view: ViewId,

    pub entries: Vec<types::Entry>,
}
