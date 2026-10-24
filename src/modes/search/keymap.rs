use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::motions::MOTIONS;
use crate::modes::search::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Cmd::Move(motion));
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('n'), KeyModifiers::empty())], Cmd::NextMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('N'), KeyModifiers::SHIFT)], Cmd::PrevMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())], Cmd::CursorsBegin);
    keymap.bind(&[KeyChord::new(KeyCode::Char('C'), KeyModifiers::SHIFT)], Cmd::CursorsEnd);

    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Cmd::Replace);

    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Cmd::Escape);

    keymap
}
