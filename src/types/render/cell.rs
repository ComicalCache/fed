use crate::types::Face;

#[derive(PartialEq, Eq)]
pub struct Cell {
    // Must be a string for multi-codepoint grapheme clusters.
    pub ch: String,
    pub width: usize,

    pub face: Face,
}

impl Cell {
    pub const fn new(ch: String, width: usize, face: Face) -> Self { Self { ch, width, face } }
}

impl Clone for Cell {
    fn clone(&self) -> Self { Self::new(self.ch.clone(), self.width, self.face) }

    fn clone_from(&mut self, source: &Self) {
        self.ch.clone_from(&source.ch);
        self.width = source.width;
        self.face = source.face;
    }
}

impl Default for Cell {
    fn default() -> Self { Self::new(" ".to_string(), 1, Face::default()) }
}
