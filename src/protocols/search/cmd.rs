use crate::decorators::SearchMatch;
use crate::protocols::search::pstate::SearchRequest;
use crate::state::DocId;
use crate::state::ViewId;

pub enum SearchCmd {
    Search {
        view: ViewId,
        doc: DocId,
        bounds: Option<Vec<(usize, usize)>>,
    },
    SearchCompletion {
        request: SearchRequest,
        query: String,
        matches: Vec<SearchMatch>,
    },
    Navigate {
        view: ViewId,
        forward: bool,
    },
    Replace {
        view: ViewId,
        doc: DocId,
        all: bool,
    },
    ReplaceCompletion {
        request: SearchRequest,
        matches: Vec<SearchMatch>,
        replacement: String,
        index: Option<usize>,
    },
    Close {
        view: ViewId,
    },

    Failed {
        request: SearchRequest,
        message: String,
    },
}
