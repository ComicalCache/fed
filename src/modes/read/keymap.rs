use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{motions::MOTIONS, read::cmd::Cmd},
    types::{KeyChord, Keymap, Motion, RectSplit},
};

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

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)], Cmd::Jump);

    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('H'), KeyModifiers::SHIFT),
        ],
        Cmd::SplitWindow(RectSplit::Horizontal),
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('V'), KeyModifiers::SHIFT),
        ],
        Cmd::SplitWindow(RectSplit::Vertical),
    );

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

    keymap.bind(&[KeyChord::new(KeyCode::Char('b'), KeyModifiers::ALT)], Cmd::EnterManageMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::ALT)], Cmd::EnterDirMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())], Cmd::EnterVisualMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())], Cmd::EnterSearchMode);

    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Cmd::Quit);

    keymap
}
