use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{dir::command::Command, motions::MOTIONS},
    types::{KeyChord, Keymap, Motion},
};

pub fn keymap() -> Keymap<Command> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Command::Move(motion));
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()), chord],
            Command::Yank(motion),
        );
    }
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()),
        ],
        Command::YankLine,
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('H'), KeyModifiers::SHIFT)],
        Command::ScrollView(Motion::Left),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('J'), KeyModifiers::SHIFT)],
        Command::ScrollView(Motion::Down),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('K'), KeyModifiers::SHIFT)],
        Command::ScrollView(Motion::Up),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('L'), KeyModifiers::SHIFT)],
        Command::ScrollView(Motion::Right),
    );

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)], Command::Jump);

    keymap.bind(&[KeyChord::new(KeyCode::Char('n'), KeyModifiers::empty())], Command::Create);
    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Command::Rename);
    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty())], Command::Delete);
    keymap
        .bind(&[KeyChord::new(KeyCode::Char('D'), KeyModifiers::SHIFT)], Command::DeleteRecursive);
    keymap.bind(&[KeyChord::new(KeyCode::Enter, KeyModifiers::NONE)], Command::Select);

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())],
        Command::EnterVisualMode,
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())],
        Command::EnterSearchMode,
    );

    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Command::Quit);

    keymap
}
