use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::motions::MOTIONS;
use crate::modes::search::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;
use crate::types::Motion;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Cmd::Move(motion));
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()), chord],
            Cmd::Yank(motion),
        );
    }
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()),
        ],
        Cmd::YankLine,
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('H'), KeyModifiers::SHIFT)],
        Cmd::ScrollView(Motion::Left),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('J'), KeyModifiers::SHIFT)],
        Cmd::ScrollView(Motion::Down),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('K'), KeyModifiers::SHIFT)],
        Cmd::ScrollView(Motion::Up),
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('L'), KeyModifiers::SHIFT)],
        Cmd::ScrollView(Motion::Right),
    );

    keymap.bind(&[KeyChord::new(KeyCode::Char('n'), KeyModifiers::empty())], Cmd::NextMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('N'), KeyModifiers::SHIFT)], Cmd::PrevMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())], Cmd::CursorsBegin);
    keymap.bind(&[KeyChord::new(KeyCode::Char('C'), KeyModifiers::SHIFT)], Cmd::CursorsEnd);

    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Cmd::Replace);
    keymap.bind(&[KeyChord::new(KeyCode::Char('R'), KeyModifiers::SHIFT)], Cmd::ReplaceAll);

    keymap.bind(&[KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL)], Cmd::SaveFile);
    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::CONTROL)], Cmd::FindFile);
    keymap.bind(&[KeyChord::new(KeyCode::Char('t'), KeyModifiers::CONTROL)], Cmd::SelectTheme);

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)], Cmd::Jump);

    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('h'), KeyModifiers::empty()),
        ],
        Cmd::FocusWindow(Motion::Left),
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('j'), KeyModifiers::empty()),
        ],
        Cmd::FocusWindow(Motion::Down),
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('k'), KeyModifiers::empty()),
        ],
        Cmd::FocusWindow(Motion::Up),
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('l'), KeyModifiers::empty()),
        ],
        Cmd::FocusWindow(Motion::Right),
    );

    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::ALT)], Cmd::EnterDirMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('b'), KeyModifiers::ALT)], Cmd::EnterManageMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Cmd::Quit);

    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Cmd::Escape);

    keymap
}
