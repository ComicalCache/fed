use std::path::PathBuf;

use crate::state::{DocId, DocStoreTypes};

#[derive(Clone)]
pub enum Event {
    Created { id: DocId },
    Destroyed { id: DocId },
    Inserted { id: DocId, pos: usize, n: usize, str: String },
    Removed { id: DocId, pos: usize, n: usize, str: String },
    Written { id: DocId, path: PathBuf, bytes_written: usize },
    ModeChanged { id: DocId, mode: DocStoreTypes::Mode },
    PathChanged { id: DocId, path: Option<PathBuf> },
}
