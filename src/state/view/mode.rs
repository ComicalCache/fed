use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Visual,
    Search,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{self:?}") }
}
