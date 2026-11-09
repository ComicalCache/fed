use crate::types::Decoration;
use crate::types::Decorator;
use crate::types::Span;

pub struct SpanDecorator {
    spans: Vec<Span<Decoration>>,
}

impl SpanDecorator {
    pub const fn new(spans: Vec<Span<Decoration>>) -> Self { Self { spans } }
}

impl Decorator for SpanDecorator {
    fn edit(&mut self, offset: usize, remove: usize, insert: usize) {
        self.spans.retain_mut(|span| {
            let zero = span.start == span.end;
            for position in [&mut span.start, &mut span.end] {
                if *position > offset {
                    *position -= remove.min(*position - offset);
                }

                if *position > offset {
                    *position += insert;
                }
            }

            zero || span.start < span.end
        });
    }

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
}
