use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    modes::{motions::MOTIONS, normal::cmd::Cmd},
    types::{KeyChord, Keymap, Motion, RectSplit},
};

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Cmd::Move(motion));
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()), chord],
            Cmd::Delete(motion),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()), chord],
            Cmd::Change(motion),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()), chord],
            Cmd::Yank(motion),
        );
    }
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()),
        ],
        Cmd::DeleteLine,
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()),
        ],
        Cmd::ChangeLine,
    );
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

    keymap.bind(&[KeyChord::new(KeyCode::Char('u'), KeyModifiers::empty())], Cmd::Undo);
    keymap.bind(&[KeyChord::new(KeyCode::Char('U'), KeyModifiers::SHIFT)], Cmd::HotRedo);

    keymap.bind(&[KeyChord::new(KeyCode::Char('a'), KeyModifiers::empty())], Cmd::Append);
    keymap.bind(&[KeyChord::new(KeyCode::Char('A'), KeyModifiers::SHIFT)], Cmd::AppendEndOfLine);
    keymap.bind(&[KeyChord::new(KeyCode::Char('o'), KeyModifiers::empty())], Cmd::InsertLineBelow);
    keymap.bind(&[KeyChord::new(KeyCode::Char('O'), KeyModifiers::SHIFT)], Cmd::InsertLineAbove);

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::ALT)], Cmd::SwapLineDown);
    keymap.bind(&[KeyChord::new(KeyCode::Char('k'), KeyModifiers::ALT)], Cmd::SwapLineUp);

    keymap.bind(&[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())], Cmd::Indent);
    keymap.bind(&[KeyChord::new(KeyCode::BackTab, KeyModifiers::SHIFT)], Cmd::Dedent);

    keymap.bind(&[KeyChord::new(KeyCode::Char('p'), KeyModifiers::empty())], Cmd::Paste);

    keymap.bind(&[KeyChord::new(KeyCode::Char('x'), KeyModifiers::empty())], Cmd::DeleteChar);
    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Cmd::Replace);

    keymap.bind(&[KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL)], Cmd::SaveFile);
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

    keymap.bind(&[KeyChord::new(KeyCode::Char('i'), KeyModifiers::empty())], Cmd::EnterInsertMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())], Cmd::EnterVisualMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())], Cmd::EnterSearchMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::ALT)], Cmd::EnterDirMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('b'), KeyModifiers::ALT)], Cmd::EnterManageMode);

    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Cmd::Quit);

    keymap
}
