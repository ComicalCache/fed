use std::collections::BTreeMap;

use crate::types::{Decoration, Decorator, Span};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum DecorationId {}

#[derive(Default)]
pub struct Decorations {
    pub decorators: BTreeMap<DecorationId, Box<dyn Decorator>>,
}

impl Decorations {
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
