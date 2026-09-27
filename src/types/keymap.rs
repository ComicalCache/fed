use std::collections::HashMap;

use crate::{debug_panic::debug_panic, types::KeyChord};

#[derive(Clone)]
pub enum KeyNode<Command: Clone> {
    Leaf(Command),
    Prefix(HashMap<KeyChord, KeyNode<Command>>),
}

pub struct Keymap<Command: Clone> {
    pub root: HashMap<KeyChord, KeyNode<Command>>,
}

impl<Command: Clone> Keymap<Command> {
    pub fn new() -> Self { Self { root: HashMap::new() } }

    pub fn bind(&mut self, chords: &[KeyChord], command: Command) {
        let mut curr = &mut self.root;
        for (idx, chord) in chords.iter().enumerate() {
            if idx == chords.len() - 1 {
                curr.insert(*chord, KeyNode::Leaf(command.clone()));
            } else {
                let node = curr.entry(*chord).or_insert_with(|| KeyNode::Prefix(HashMap::new()));
                if let KeyNode::Prefix(next_map) = node {
                    curr = next_map;
                } else {
                    debug_panic!();
                    return;
                }
            }
        }
    }
}
