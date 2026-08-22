use std::path::PathBuf;

use piece_table::PieceTable;

pub type DocumentId = u64;

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
