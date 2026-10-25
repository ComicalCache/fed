use std::collections::HashMap;

use crate::state::DocId;
use crate::state::ViewId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SearchRequest {
    pub request_id: u64,

    pub view: ViewId,
    pub doc: DocId,
    pub revision: u64,
}

#[derive(Default)]
pub struct SearchPState {
    pub next_id: u64,
    pub searches: HashMap<ViewId, SearchRequest>,
}
