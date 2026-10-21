use std::path::PathBuf;

use piece_table::PieceTable;

#[derive(Default)]
pub struct Doc {
    pub path: Option<PathBuf>,

    pub data: PieceTable,
    pub modified: bool,
    pub revision: u64,
}

impl Doc {
    pub const fn new(path: Option<PathBuf>, data: PieceTable) -> Self {
        Self { path, data, revision: 0, modified: false }
    }
}
