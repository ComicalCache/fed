use std::any::Any;

use crate::types::{Decoration, Decorator, Span};

pub struct SpanDecorator {
    spans: Vec<Span<Decoration>>,
}

impl SpanDecorator {
    pub const fn new(spans: Vec<Span<Decoration>>) -> Self { Self { spans } }
}

impl Decorator for SpanDecorator {
    fn edit(&mut self, _: usize, _: usize, _: usize) {}

    fn update(&mut self, _: usize, _: usize) {}

    fn range(&self, start: usize, end: usize, callback: &mut dyn FnMut(Span<Decoration>)) {
        for span in &self.spans {
            if span.start < end && span.end > start {
                callback(Span {
                    start: span.start.max(start),
                    end: span.end.min(end),
                    data: span.data.clone(),
                });
            }
        }
    }

    fn any(&self) -> &dyn Any { self }

    fn any_mut(&mut self) -> &mut dyn Any { self }
}
