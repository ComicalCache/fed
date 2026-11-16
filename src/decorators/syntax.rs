use crate::types::Decoration;
use crate::types::Decorator;
use crate::types::Span;
use crate::types::SyntaxHighlight;

pub struct SyntaxDecorator {
    pub spans: Vec<Span<SyntaxHighlight>>,
}

impl Decorator for SyntaxDecorator {
    fn edit(&mut self, offset: usize, remove: usize, insert: usize) {
        // Shift highlights until the new ones are calculated and updated.
        self.spans.retain_mut(|span| {
            for position in [&mut span.start, &mut span.end] {
                if *position > offset {
                    *position -= remove.min(*position - offset);
                }
            }

            if span.start >= span.end {
                return false;
            }

            if span.start >= offset {
                span.start += insert;
            }
            if span.end > offset {
                span.end += insert;
            }

            true
        });
    }

    fn update(&mut self, _: usize, _: usize) {}

    fn range(&self, start: usize, end: usize, callback: &mut dyn FnMut(Span<Decoration>)) {
        if start >= end {
            return;
        }

        let first = self.spans.partition_point(|s| s.end <= start);
        for span in self.spans[first..].iter().take_while(|span| span.start < end) {
            callback(Span {
                start: span.start.max(start),
                end: span.end.min(end),
                data: Decoration::Syntax { highlight: span.data },
            });
        }
    }
}
