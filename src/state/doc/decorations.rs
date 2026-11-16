#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum DecorationId {
    TreeSitter,
    Span,
    Directory,
}

pub type Decorations = crate::types::Decorations<DecorationId>;
