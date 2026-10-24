use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Text,
    Mp,
    Dir,
    Manage,
    Picker,

    Read,
}

impl Mode {
    pub const fn singleton(self) -> bool {
        matches!(self, Self::Mp | Self::Dir | Self::Manage | Self::Picker)
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{self:?}") }
}
