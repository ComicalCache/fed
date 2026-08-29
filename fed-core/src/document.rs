use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use piece_table::PieceTable;

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(pub u64);

impl DerefMut for DocumentId {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for DocumentId {
    type Target = u64;

    fn deref(&self) -> &Self::Target { &self.0 }
}

pub(crate) struct Document {
    pub(crate) path: Option<PathBuf>,

    pub(crate) data: PieceTable,
    pub(crate) modified: bool,
}

impl Document {
    pub(crate) fn new(path: Option<PathBuf>, data: PieceTable) -> Self {
        Self { path, data, modified: false }
    }
}
