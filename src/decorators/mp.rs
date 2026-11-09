use crate::types::Decoration;
use crate::types::Decorator;
use crate::types::Face;
use crate::types::Span;

pub struct MpDecorator {
    pub prompt: String,
}

impl MpDecorator {
    pub const fn new(prompt: String) -> Self { Self { prompt } }
}

impl Decorator for MpDecorator {
    fn edit(&mut self, _: usize, _: usize, _: usize) {}

    fn update(&mut self, _: usize, _: usize) {}

    fn range(&self, start: usize, _: usize, callback: &mut dyn FnMut(Span<Decoration>)) {
        if start != 0 {
            return;
        }

        callback(Span {
            start,
            end: 0,
            data: Decoration::VirtualText { text: self.prompt.clone(), face: Face::default() },
        });
    }
}
