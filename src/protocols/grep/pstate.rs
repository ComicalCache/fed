use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use crate::state::DocId;
use crate::state::ViewId;

mod document;
mod file;
mod scan;

pub use document::GrepDocument;
pub use file::GrepFile;
pub use scan::GrepScan;

#[derive(Clone, Copy)]
pub struct GrepRow {
    pub file: usize,
    pub line: usize,
}

#[derive(Clone, Copy)]
pub struct GrepMatch {
    pub file: usize,
    pub r#match: usize,

    pub start: usize,
    pub end: usize,
}

#[derive(Default)]
pub struct GrepPState {
    pub doc: DocId,
    pub view: ViewId,

    pub root: PathBuf,
    pub query: String,
    pub replacement: Option<String>,

    pub files: Vec<GrepFile>,
    pub rows: Vec<Option<GrepRow>>,
    pub matches: Vec<GrepMatch>,
    pub revision: Arc<AtomicU64>,
}

impl GrepPState {
    pub fn next_pending_match(&self, start: usize) -> Option<&GrepMatch> {
        self.matches
            .iter()
            .cycle()
            .skip(start)
            .take(self.matches.len())
            .find(|m| self.files[m.file].matches[m.r#match].replacement.is_none())
    }
}
