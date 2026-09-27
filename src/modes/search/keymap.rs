use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{motions::MOTIONS, search::command::Command},
    types::{KeyChord, Keymap},
};

pub fn keymap() -> Keymap<Command> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Command::Move(motion));
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('n'), KeyModifiers::empty())], Command::NextMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('N'), KeyModifiers::SHIFT)], Command::PrevMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())], Command::CursorsBegin);
    keymap.bind(&[KeyChord::new(KeyCode::Char('C'), KeyModifiers::SHIFT)], Command::CursorsEnd);

    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Command::Replace);

    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Command::Escape);

    keymap
}
