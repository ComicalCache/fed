use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{motions::MOTIONS, visual::cmd::Cmd},
    types::{KeyChord, Keymap},
};

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Cmd::Move(motion));
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty())], Cmd::Delete);
    keymap.bind(&[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())], Cmd::Change);
    keymap.bind(&[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty())], Cmd::Yank);
    keymap.bind(&[KeyChord::new(KeyCode::Char('p'), KeyModifiers::empty())], Cmd::Paste);

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::ALT)], Cmd::SwapLineDown);
    keymap.bind(&[KeyChord::new(KeyCode::Char('k'), KeyModifiers::ALT)], Cmd::SwapLineUp);

    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())], Cmd::EnterSearchMode);

    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Cmd::Escape);

    keymap
}
