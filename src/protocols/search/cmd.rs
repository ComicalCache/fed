use crate::protocols::search::pstate::SearchRequest;
use crate::state::DocId;
use crate::state::ViewId;

pub enum SearchCmd {
    Search { view: ViewId, doc: DocId, bounds: Option<Vec<(usize, usize)>> },
    SearchCompletion { request: SearchRequest, query: String, matches: Vec<(usize, usize)> },
    Replace { view: ViewId, doc: DocId },
    ReplaceCompletion { request: SearchRequest, edits: Vec<(usize, usize, String)> },
    Close { view: ViewId },

    Failed { request: SearchRequest, message: String },
}
