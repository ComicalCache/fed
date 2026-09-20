use std::path::PathBuf;

use piece_table::PieceTable;

use crate::state::DocumentId;

#[derive(Clone)]
pub enum DocumentEvent {
    Created { id: DocumentId },
    Destroyed { id: DocumentId },
    Inserted { id: DocumentId, pos: usize, n: usize, str: String },
    Removed { id: DocumentId, pos: usize, n: usize, str: String },
    Written { id: DocumentId, path: PathBuf, bytes_written: usize },
}

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
