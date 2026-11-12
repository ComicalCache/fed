use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

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

impl Display for KeyChord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut modifiers = self.modifiers;

        match self.code {
            KeyCode::BackTab => modifiers.remove(KeyModifiers::SHIFT),
            KeyCode::Char(ch) if ch.is_uppercase() || ch.is_ascii_punctuation() => {
                modifiers.remove(KeyModifiers::SHIFT);
            }
            _ => {}
        }

        if let KeyCode::Char(ch) = self.code
            && modifiers.is_empty()
            && !ch.is_whitespace()
            && !ch.is_control()
        {
            return write!(f, "{ch}");
        }

        f.write_str("<")?;
        for (modifier, name) in [
            (KeyModifiers::CONTROL, "C"),
            (KeyModifiers::ALT, "A"),
            (KeyModifiers::SHIFT, "S"),
            (KeyModifiers::SUPER, "Super"),
            (KeyModifiers::HYPER, "Hyper"),
            (KeyModifiers::META, "Meta"),
        ] {
            if modifiers.contains(modifier) {
                write!(f, "{name}-")?;
            }
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("Space")?,
            KeyCode::Char(ch) if ch.is_control() || ch.is_whitespace() => {
                write!(f, "U+{:04X}", u32::from(ch))?;
            }
            KeyCode::Char(ch) => write!(f, "{ch}")?,
            KeyCode::F(number) => write!(f, "F{number}")?,
            KeyCode::Media(key) => write!(f, "Media{key:?}")?,
            KeyCode::Modifier(key) => write!(f, "{key:?}")?,
            code => write!(f, "{code:?}")?,
        }
        f.write_str(">")
    }
}

#[derive(Clone)]
enum KeyNode<Command: Clone> {
    Leaf(Command, &'static str),
    Prefix(HashMap<KeyChord, Self>),
}

pub struct Keymap<Command: Clone> {
    root: HashMap<KeyChord, KeyNode<Command>>,
}

impl<Command: Clone> Keymap<Command> {
    pub fn new() -> Self { Self { root: HashMap::new() } }

    pub fn bind(&mut self, chords: &[KeyChord], command: Command, synopsis: &'static str) {
        let mut curr = &mut self.root;
        for (idx, chord) in chords.iter().enumerate() {
            if idx == chords.len() - 1 {
                curr.insert(*chord, KeyNode::Leaf(command.clone(), synopsis));
            } else {
                let node = curr.entry(*chord).or_insert_with(|| KeyNode::Prefix(HashMap::new()));
                if let KeyNode::Prefix(next_map) = node {
                    curr = next_map;
                } else {
                    panic!();
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
                    return ParseResult::Invalid;
                }
            } else {
                return ParseResult::Invalid;
            }
        }

        match target {
            Some(KeyNode::Leaf(cmd, _)) => ParseResult::Exact(cmd.clone()),
            Some(KeyNode::Prefix(_)) => ParseResult::Prefix,
            None => ParseResult::Invalid,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapHelpBinding {
    pub sequence: String,
    pub command: String,
    pub synopsis: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapHelp {
    pub mode: &'static str,
    pub bindings: Vec<KeymapHelpBinding>,
}

impl<Command: Clone + Debug> Keymap<Command> {
    pub fn help(&self, mode: &'static str) -> KeymapHelp {
        let mut bindings = Vec::new();

        Self::collect_bindings(&self.root, "", &mut bindings);

        bindings.sort_by(|a, b| {
            a.command
                .cmp(&b.command)
                .then(a.sequence.cmp(&b.sequence))
                .then(a.synopsis.cmp(b.synopsis))
        });

        KeymapHelp { mode, bindings }
    }

    fn collect_bindings(
        nodes: &HashMap<KeyChord, KeyNode<Command>>, prefix: &str,
        bindings: &mut Vec<KeymapHelpBinding>,
    ) {
        for (chord, node) in nodes {
            let sequence =
                if prefix.is_empty() { chord.to_string() } else { format!("{prefix} {chord}") };

            match node {
                KeyNode::Leaf(command, synopsis) => bindings.push(KeymapHelpBinding {
                    sequence,
                    command: format!("{command:?}"),
                    synopsis,
                }),
                KeyNode::Prefix(children) => Self::collect_bindings(children, &sequence, bindings),
            }
        }
    }
}
