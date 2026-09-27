use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::insert::command::Command,
    types::{KeyChord, Keymap, Motion},
};

pub fn keymap() -> Keymap<Command> {
    let mut keymap = Keymap::new();

    keymap.bind(&[KeyChord::new(KeyCode::Up, KeyModifiers::empty())], Command::Move(Motion::Up));
    keymap
        .bind(&[KeyChord::new(KeyCode::Down, KeyModifiers::empty())], Command::Move(Motion::Down));
    keymap
        .bind(&[KeyChord::new(KeyCode::Left, KeyModifiers::empty())], Command::Move(Motion::Left));
    keymap.bind(
        &[KeyChord::new(KeyCode::Right, KeyModifiers::empty())],
        Command::Move(Motion::Right),
    );

    keymap
        .bind(&[KeyChord::new(KeyCode::Left, KeyModifiers::ALT)], Command::Move(Motion::PrevWord));
    keymap.bind(
        &[KeyChord::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::ALT)],
        Command::Move(Motion::PrevWordEnd),
    );
    keymap
        .bind(&[KeyChord::new(KeyCode::Right, KeyModifiers::ALT)], Command::Move(Motion::NextWord));
    keymap.bind(
        &[KeyChord::new(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::ALT)],
        Command::Move(Motion::NextWordEnd),
    );

    keymap.bind(&[KeyChord::new(KeyCode::Backspace, KeyModifiers::empty())], Command::Backspace);
    keymap.bind(&[KeyChord::new(KeyCode::Delete, KeyModifiers::empty())], Command::Delete);

    keymap.bind(
        &[KeyChord::new(KeyCode::Enter, KeyModifiers::empty())],
        Command::Input("\n".to_string()),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())],
        Command::Input("\t".to_string()),
    );
    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Command::Escape);

    keymap
}
