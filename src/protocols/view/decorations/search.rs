use std::any::Any;

use crate::types::{Decoration, DecorationProvider, Face, Span};

pub struct SearchDecorationProvider {
    pub query: String,
    pub matches: Vec<(usize, usize)>,

    pub face: Face,
}

impl SearchDecorationProvider {
    pub fn new(query: String, matches: Vec<(usize, usize)>, face: Face) -> Self {
        Self { query, matches, face }
    }
}

impl DecorationProvider for SearchDecorationProvider {
    fn edit(&mut self, offset: usize, remove: usize, insert: usize) {
        for m in &mut self.matches {
            if m.0 > offset {
                m.0 = if m.0 < offset + remove { offset } else { m.0 - remove + insert };
            }
            if m.1 > offset {
                m.1 = if m.1 < offset + remove { offset } else { m.1 - remove + insert };
            }
        }

        self.matches.retain(|m| m.0 < m.1);
    }

    fn update(&mut self, _: usize, _: usize) {}

    fn range(&self, start: usize, end: usize, callback: &mut dyn FnMut(Span<Decoration>)) {
        for &(match_start, match_end) in &self.matches {
            if match_start < end && match_end > start {
                callback(Span {
                    start: match_start.max(start),
                    end: match_end.min(end),
                    data: Decoration::Style { face: self.face },
                });
            }
        }
    }

    fn any(&self) -> &dyn Any { self }

    fn any_mut(&mut self) -> &mut dyn Any { self }
}
