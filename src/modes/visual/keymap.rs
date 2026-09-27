use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{motions::MOTIONS, visual::command::Command},
    types::{KeyChord, Keymap},
};

pub fn keymap() -> Keymap<Command> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Command::Move(motion));
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty())], Command::Delete);
    keymap.bind(&[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())], Command::Change);
    keymap.bind(&[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty())], Command::Yank);
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())],
        Command::EnterSearchMode,
    );

    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Command::Escape);

    keymap
}
