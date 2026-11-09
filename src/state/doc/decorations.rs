#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum DecorationId {
    Span,
    Directory,
}

pub type Decorations = crate::types::Decorations<DecorationId>;
