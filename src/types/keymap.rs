use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::debug_panic::debug_panic;

pub enum ParseResult<Command: Clone> {
    Exact(Command),
    Prefix,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyChord {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self { Self { code, modifiers } }
}

impl From<&KeyEvent> for KeyChord {
    fn from(event: &KeyEvent) -> Self { Self { code: event.code, modifiers: event.modifiers } }
}

#[derive(Clone)]
enum KeyNode<Command: Clone> {
    Leaf(Command),
    Prefix(HashMap<KeyChord, KeyNode<Command>>),
}

pub struct Keymap<Command: Clone> {
    root: HashMap<KeyChord, KeyNode<Command>>,
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

    pub fn parse(&self, chords: &[KeyChord]) -> ParseResult<Command> {
        let mut curr = &self.root;
        let mut target = None;

        for (idx, key_chord) in chords.iter().enumerate() {
            if let Some(node) = curr.get(key_chord) {
                if idx == chords.len() - 1 {
                    target = Some(node);
                } else if let KeyNode::Prefix(next_map) = node {
                    curr = next_map;
                } else {
                    debug_panic!();
                }
            } else {
                return ParseResult::Invalid;
            }
        }

        match target {
            Some(KeyNode::Leaf(cmd)) => ParseResult::Exact(cmd.clone()),
            Some(KeyNode::Prefix(_)) => ParseResult::Prefix,
            None => ParseResult::Invalid,
        }
    }
}
