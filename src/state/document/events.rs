use std::path::PathBuf;

use crate::state::DocumentId;

#[derive(Clone)]
pub enum DocumentEvent {
    Created { id: DocumentId },
    Destroyed { id: DocumentId },
    Inserted { id: DocumentId, pos: usize, n: usize, str: String },
    Removed { id: DocumentId, pos: usize, n: usize, str: String },
    Written { id: DocumentId, path: PathBuf, bytes_written: usize },
}
