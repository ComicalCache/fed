use tree_sitter_highlight::Highlighter;

use crate::state::DocId;
use crate::types::Span;
use crate::types::SyntaxHighlight;

pub enum TreeSitterCmd {
    HighlightCompletion {
        doc: DocId,
        highlighter: Box<Highlighter>,
        result: Result<Vec<Span<SyntaxHighlight>>, String>,
        r#gen: u64,
        rev: u64,
    },
}
