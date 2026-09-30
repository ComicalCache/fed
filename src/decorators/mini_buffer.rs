use std::any::Any;

use crate::types::{Decoration, Decorator, Face, Span};

pub struct MiniBufferDecorator {
    pub prompt: String,
    pub face: Face,
}

impl MiniBufferDecorator {
    pub fn new(prompt: String, face: Face) -> Self { Self { prompt, face } }
}

impl Decorator for MiniBufferDecorator {
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
