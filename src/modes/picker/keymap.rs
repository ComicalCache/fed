use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::picker::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;
use crate::types::Motion;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    keymap.bind(
        &[KeyChord::new(KeyCode::Left, KeyModifiers::empty())],
        Cmd::Move(Motion::Left),
        "Move the cursor with the motion",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Right, KeyModifiers::empty())],
        Cmd::Move(Motion::Right),
        "Move the cursor with the motion",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Left, KeyModifiers::CONTROL)],
        Cmd::Move(Motion::PrevWord),
        "Move to the previous word",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Right, KeyModifiers::CONTROL)],
        Cmd::Move(Motion::NextWordEnd),
        "Move to the next word end",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('h'), KeyModifiers::CONTROL)],
        Cmd::ScrollList(Motion::Left),
        "Scroll the list",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('l'), KeyModifiers::CONTROL)],
        Cmd::ScrollList(Motion::Right),
        "Scroll the list",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Up, KeyModifiers::empty())],
        Cmd::ChangeSelection(Motion::Up),
        "Move the selection up",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Down, KeyModifiers::empty())],
        Cmd::ChangeSelection(Motion::Down),
        "Move the selection down",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('k'), KeyModifiers::CONTROL)],
        Cmd::ChangeSelection(Motion::Up),
        "Move the selection up",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)],
        Cmd::ChangeSelection(Motion::Down),
        "Move the selection down",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Backspace, KeyModifiers::empty())],
        Cmd::Backspace,
        "Delete the character before the cursor",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Delete, KeyModifiers::empty())],
        Cmd::Delete,
        "Delete the character under the cursor",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Backspace, KeyModifiers::CONTROL)],
        Cmd::DeleteMotion(Motion::PrevWord),
        "Delete the previous word",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Delete, KeyModifiers::CONTROL)],
        Cmd::DeleteMotion(Motion::NextWordEnd),
        "Delete the next word",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())],
        Cmd::Input("\t".to_string()),
        "Insert a tab character",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Enter, KeyModifiers::empty())],
        Cmd::Select,
        "Select the current item",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())],
        Cmd::Close,
        "Close the selector",
    );

    keymap
}
