use crate::types::Face;

#[derive(Clone, PartialEq, Eq)]
pub struct Cell {
    // Must be a string for multi-codepoint grapheme clusters.
    pub ch: String,
    pub width: usize,

    pub face: Face,
}

impl Cell {
    pub fn new(ch: String, width: usize, face: Face) -> Self { Self { ch, width, face } }
}

impl Default for Cell {
    fn default() -> Self { Self::new(" ".to_string(), 1, Face::default()) }
}
