use std::collections::BTreeMap;

use crate::types::Face;
use crate::types::Span;

#[derive(Clone, PartialEq, Eq)]
#[allow(unused)]
pub enum Decoration {
    /// Modifies the face of the rendered text.
    Style { face: Face },
    /// Replaces the underlying text with new one.
    Replace { text: String, face: Face },
    /// Inserts "virtual text" without replacing/consuming the underlying text.
    VirtualText { text: String, face: Face },
}

pub trait Decorator: Send + Sync + 'static {
    /// Called during incremental changes.
    fn edit(&mut self, offset: usize, remove: usize, insert: usize);

    /// Called _before_ rendering with the visible region on the screen.
    fn update(&mut self, start: usize, end: usize);

    /// Yields Decorations for a range.
    fn range(&self, start: usize, end: usize, callback: &mut dyn FnMut(Span<Decoration>));
}

pub struct Decorations<Id> {
    pub decorators: BTreeMap<Id, Box<dyn Decorator>>,
}

impl<Id> Default for Decorations<Id> {
    fn default() -> Self { Self { decorators: BTreeMap::new() } }
}

impl<Id> Decorations<Id> {
    pub fn edit(&mut self, offset: usize, remove: usize, insert: usize) {
        for provider in self.decorators.values_mut() {
            provider.edit(offset, remove, insert);
        }
    }

    pub fn update(&mut self, start: usize, end: usize) {
        for provider in self.decorators.values_mut() {
            provider.update(start, end);
        }
    }

    pub fn range(&self, start: usize, end: usize, buff: &mut Vec<Span<Decoration>>) {
        for provider in self.decorators.values() {
            provider.range(start, end, &mut |s| buff.push(s));
        }
    }
}
