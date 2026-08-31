use crate::types::Face;

/// Decorations modify the styling and rendering of text.
#[derive(Clone, PartialEq, Eq)]
pub enum Decoration {
    /// Modifies the face of the rendered text.
    Style(Face),
    /// Replaces the underlying text with new one.
    Replace { text: String, face: Face },
    /// Inserts "virtual text" without replacing/consuming the underlying text.
    VirtualText { text: String, face: Face },
}
