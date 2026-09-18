use std::any::Any;

use crate::types::{Face, Span};

#[derive(Clone, PartialEq, Eq)]
pub enum Decoration {
    /// Modifies the face of the rendered text.
    Style { face: Face },
    /// Replaces the underlying text with new one.
    Replace { text: String, face: Face },
    /// Inserts "virtual text" without replacing/consuming the underlying text.
    VirtualText { text: String, face: Face },
}

pub trait DecorationProvider: Send + Sync + 'static {
    /// Called during incremental changes.
    fn edit(&mut self, offset: usize, remove: usize, insert: usize);

    /// Called _before_ rendering with the visible region on the screen.
    fn update(&mut self, start: usize, end: usize);

    /// Yields Decorations for a range.
    fn range(&self, start: usize, end: usize, callback: &mut dyn FnMut(Span<Decoration>));

    fn any(&self) -> &dyn Any;

    fn any_mut(&mut self) -> &mut dyn Any;
}
