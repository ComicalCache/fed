use std::any::Any;

use crate::types::Decoration;
use crate::types::Decorator;
use crate::types::Face;
use crate::types::Span;

#[derive(Clone)]
pub struct SearchMatch {
    pub start: usize,
    pub end: usize,
    pub replacement: Option<String>,
}

pub struct SearchDecorator {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub revision: u64,

    pub replacement: Option<String>,
    pub replaced_at: Option<usize>,

    pub face: Face,
}

impl SearchDecorator {
    pub const fn new(query: String, matches: Vec<SearchMatch>, face: Face, revision: u64) -> Self {
        Self { query, matches, revision, replacement: None, replaced_at: None, face }
    }
}

impl Decorator for SearchDecorator {
    fn edit(&mut self, offset: usize, remove: usize, insert: usize) {
        for m in &mut self.matches {
            if m.start > offset {
                m.start =
                    if m.start < offset + remove { offset } else { m.start - remove + insert };
            }
            if m.end > offset {
                m.end = if m.end < offset + remove { offset } else { m.end - remove + insert };
            }
        }

        self.matches.retain(|m| m.start < m.end);
    }

    fn update(&mut self, _: usize, _: usize) {}

    fn range(&self, start: usize, end: usize, callback: &mut dyn FnMut(Span<Decoration>)) {
        for m in &self.matches {
            if m.start < end && m.end > start {
                callback(Span {
                    start: m.start.max(start),
                    end: m.end.min(end),
                    data: Decoration::Style { face: self.face },
                });
            }
        }
    }

    fn any(&self) -> &dyn Any { self }

    fn any_mut(&mut self) -> &mut dyn Any { self }
}
