#[derive(Clone)]
pub struct SearchMatch {
    pub start: usize,
    pub end: usize,
    pub replacement: Option<String>,
}

pub struct SearchState {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub last_match: Option<usize>,
    pub revision: u64,

    pub replacement: Option<String>,
}

impl SearchState {
    pub const fn new(query: String, matches: Vec<SearchMatch>, revision: u64) -> Self {
        Self { query, matches, last_match: None, revision, replacement: None }
    }
}
