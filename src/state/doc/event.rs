use std::path::PathBuf;

use crate::state::DocId;
use crate::state::DocStoreTypes;

#[derive(Clone)]
#[allow(unused)]
pub enum Event {
    Created { doc: DocId },
    Destroyed { doc: DocId },
    Inserted { doc: DocId, pos: usize, n: usize, str: String },
    Removed { doc: DocId, pos: usize, n: usize, str: String },
    Written { doc: DocId, path: PathBuf, bytes_written: usize },
    ModeChanged { doc: DocId, mode: DocStoreTypes::Mode },
    PathChanged { doc: DocId, path: Option<PathBuf> },
}
