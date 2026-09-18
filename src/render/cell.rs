use crate::types::Face;

#[derive(Clone, PartialEq, Eq)]
pub struct Cell {
    // Must be a string for multi-codepoint grapheme clusters.
    pub ch: String,
    pub wide_trailing: bool,

    pub face: Face,
}

impl Cell {
    pub fn new(ch: String, wide_trailing: bool, face: Face) -> Self {
        Self { ch, wide_trailing, face }
    }
}

impl Default for Cell {
    fn default() -> Self { Self::new(" ".to_string(), false, Face::default()) }
}
