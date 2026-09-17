use std::any::Any;

use crate::types::{Decoration, DecorationProvider, Face, Span};

pub struct MiniBufferDecorationProvider {
    pub prompt: String,
    pub face: Face,
}

impl MiniBufferDecorationProvider {
    pub fn new(prompt: String, face: Face) -> Self { Self { prompt, face } }
}

impl DecorationProvider for MiniBufferDecorationProvider {
    fn edit(&mut self, _: usize, _: usize, _: usize) {}

    fn update(&mut self, _: usize, _: usize) {}

    fn range(&self, start: usize, _: usize, callback: &mut dyn FnMut(Span<Decoration>)) {
        if start != 0 {
            return;
        }

        callback(Span {
            start,
            end: 0,
            data: Decoration::VirtualText { text: self.prompt.clone(), face: self.face },
        })
    }

    fn any(&self) -> &dyn Any { self }

    fn any_mut(&mut self) -> &mut dyn Any { self }
}
