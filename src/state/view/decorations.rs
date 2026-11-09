#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum DecorationId {
    Mp,
}

pub type Decorations = crate::types::Decorations<DecorationId>;
