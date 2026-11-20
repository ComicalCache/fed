use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::insert::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;
use crate::types::Motion;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    keymap.bind(
        &[KeyChord::new(KeyCode::Up, KeyModifiers::empty())],
        Cmd::Move(Motion::Up),
        "Move the cursor with the motion",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Down, KeyModifiers::empty())],
        Cmd::Move(Motion::Down),
        "Move the cursor with the motion",
    );
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
        &[KeyChord::new(KeyCode::Enter, KeyModifiers::empty())],
        Cmd::Input("\n".to_string()),
        "Insert a newline",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())],
        Cmd::Tab,
        "Insert indentation",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
        Cmd::SaveDoc,
        "Save the document",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('f'), KeyModifiers::CONTROL)],
        Cmd::FindFile,
        "Open the file selector",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('t'), KeyModifiers::CONTROL)],
        Cmd::SelectTheme,
        "Open the theme selector",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('i'), KeyModifiers::CONTROL)],
        Cmd::ToggleIndentation,
        "Toggle between tabs and spaces for indentation",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())],
        Cmd::Escape,
        "Exit insert mode",
    );

    keymap
}
