mod entry;
mod mode_line;

pub mod types {
    pub use crate::state::dir::{
        entry::{Entry, EntryKind},
        mode_line::DirModeLine,
    };
}

use std::path::PathBuf;

use crate::state::{DocId, ViewId};

#[derive(Default)]
pub struct Dir {
    pub doc: DocId,
    pub view: ViewId,
    pub pwd: PathBuf,
    pub entries: Vec<types::Entry>,
}
