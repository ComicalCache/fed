use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{motions::MOTIONS, normal::command::Command},
    types::{KeyChord, Keymap, Motion},
};

pub fn keymap() -> Keymap<Command> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Command::Move(motion));
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()), chord],
            Command::Delete(motion),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()), chord],
            Command::Change(motion),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()), chord],
            Command::Yank(motion),
        );
    }
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()),
        ],
        Command::DeleteLine,
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()),
        ],
        Command::ChangeLine,
    );
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

    keymap.bind(&[KeyChord::new(KeyCode::Char('u'), KeyModifiers::empty())], Command::Undo);
    keymap.bind(&[KeyChord::new(KeyCode::Char('U'), KeyModifiers::SHIFT)], Command::HotRedo);

    keymap.bind(&[KeyChord::new(KeyCode::Char('a'), KeyModifiers::empty())], Command::Append);
    keymap
        .bind(&[KeyChord::new(KeyCode::Char('A'), KeyModifiers::SHIFT)], Command::AppendEndOfLine);
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('o'), KeyModifiers::empty())],
        Command::InsertLineBelow,
    );
    keymap
        .bind(&[KeyChord::new(KeyCode::Char('O'), KeyModifiers::SHIFT)], Command::InsertLineAbove);

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::ALT)], Command::SwapLineDown);
    keymap.bind(&[KeyChord::new(KeyCode::Char('k'), KeyModifiers::ALT)], Command::SwapLineUp);

    keymap.bind(&[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())], Command::Indent);
    keymap.bind(&[KeyChord::new(KeyCode::BackTab, KeyModifiers::SHIFT)], Command::Dedent);

    keymap.bind(&[KeyChord::new(KeyCode::Char('p'), KeyModifiers::empty())], Command::Paste);

    keymap.bind(&[KeyChord::new(KeyCode::Char('x'), KeyModifiers::empty())], Command::DeleteChar);
    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Command::Replace);

    keymap.bind(&[KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL)], Command::SaveFile);
    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)], Command::Jump);

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('i'), KeyModifiers::empty())],
        Command::EnterInsertMode,
    );
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
