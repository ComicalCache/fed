use std::collections::HashMap;
use std::path::PathBuf;

use crate::protocols::grep::pstate::GrepDocument;
use crate::protocols::grep::pstate::GrepScan;
use crate::state::ViewId;
use crate::types::render::WindowId;

#[allow(unused)]
pub enum GrepCmd {
    Search {
        window: WindowId,
    },
    SearchCompletion {
        window: WindowId,
        views: Vec<ViewId>,
        query: String,
        result: Result<(GrepScan, GrepDocument), String>,
        revision: u64,
    },
    Show {
        window: WindowId,
    },
    Navigate {
        forward: bool,
    },
    SelectResult,
    SelectResultCompletion {
        window: WindowId,
        file: usize,
        position: (usize, usize),
        result: Result<String, String>,
        revision: u64,
    },
    Replace {
        all: bool,
    },
    ReplaceCompletion {
        target: Option<(PathBuf, usize)>,
        replacement: String,
        result: Result<HashMap<PathBuf, String>, String>,
        revision: u64,
    },
    Close,
}
