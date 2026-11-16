use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use tokio::sync::Semaphore;
use tree_sitter_highlight::HighlightConfiguration;
use tree_sitter_highlight::Highlighter;

use crate::state::DocId;

pub(super) struct TreeSitterDoc {
    pub highlighter: Option<Box<Highlighter>>,
    pub config: Arc<HighlightConfiguration>,
    pub cancellation: Arc<AtomicUsize>,
    pub r#gen: u64,
    pub rev: Option<u64>,
}

pub struct TreeSitterPState {
    pub(super) docs: HashMap<DocId, TreeSitterDoc>,
    pub(super) next_gen: u64,

    pub(super) workers: Arc<Semaphore>,
}

impl Default for TreeSitterPState {
    fn default() -> Self {
        Self { docs: HashMap::new(), next_gen: 0, workers: Arc::new(Semaphore::new(2)) }
    }
}
