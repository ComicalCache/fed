use std::path::PathBuf;

use piece_table::PieceTable;

#[derive(Default)]
pub struct Document {
    pub path: Option<PathBuf>,

    pub data: PieceTable,
    pub modified: bool,
}

impl Document {
    pub fn new(path: Option<PathBuf>, data: PieceTable) -> Self {
        Self { path, data, modified: false }
    }
}
