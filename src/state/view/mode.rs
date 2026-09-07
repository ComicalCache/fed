use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{:?}", self) }
}
