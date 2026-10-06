use crate::newtype::newtype;

newtype!(TabWidth, usize, Clone, Copy, PartialEq, Eq, Hash);

impl Default for TabWidth {
    fn default() -> Self { Self(4) }
}
