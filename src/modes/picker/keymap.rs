use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::picker::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;
use crate::types::Motion;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    keymap.bind(&[KeyChord::new(KeyCode::Left, KeyModifiers::empty())], Cmd::Move(Motion::Left));
    keymap.bind(&[KeyChord::new(KeyCode::Right, KeyModifiers::empty())], Cmd::Move(Motion::Right));
    keymap.bind(&[KeyChord::new(KeyCode::Left, KeyModifiers::ALT)], Cmd::Move(Motion::PrevWord));
    keymap.bind(&[KeyChord::new(KeyCode::Right, KeyModifiers::ALT)], Cmd::Move(Motion::NextWord));

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('h'), KeyModifiers::CONTROL)],
        Cmd::ScrollList(Motion::Left),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('l'), KeyModifiers::CONTROL)],
        Cmd::ScrollList(Motion::Right),
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('k'), KeyModifiers::CONTROL)],
        Cmd::ChangeSelection(Motion::Up),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)],
        Cmd::ChangeSelection(Motion::Down),
    );

    keymap.bind(&[KeyChord::new(KeyCode::Backspace, KeyModifiers::empty())], Cmd::Backspace);
    keymap.bind(&[KeyChord::new(KeyCode::Delete, KeyModifiers::empty())], Cmd::Delete);
    keymap
        .bind(&[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())], Cmd::Input("\t".to_string()));

    keymap.bind(&[KeyChord::new(KeyCode::Enter, KeyModifiers::empty())], Cmd::Select);
    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Cmd::Close);

    keymap
}
