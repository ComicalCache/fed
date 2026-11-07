use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::motions::MOTIONS;
use crate::modes::visual::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;
use crate::types::Motion;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Cmd::Move(motion));
    }

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

    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty())], Cmd::Delete);
    keymap.bind(&[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())], Cmd::Change);
    keymap.bind(&[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty())], Cmd::Yank);
    keymap.bind(&[KeyChord::new(KeyCode::Char('p'), KeyModifiers::empty())], Cmd::Paste);

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::ALT)], Cmd::SwapLineDown);
    keymap.bind(&[KeyChord::new(KeyCode::Char('k'), KeyModifiers::ALT)], Cmd::SwapLineUp);

    keymap.bind(&[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())], Cmd::Indent);
    keymap.bind(&[KeyChord::new(KeyCode::BackTab, KeyModifiers::SHIFT)], Cmd::Dedent);
    keymap.bind(&[KeyChord::new(KeyCode::Char('o'), KeyModifiers::CONTROL)], Cmd::Sort);
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('o'), KeyModifiers::CONTROL | KeyModifiers::SHIFT)],
        Cmd::SortReverse,
    );

    keymap.bind(&[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())], Cmd::EnterVisualMode);
    keymap
        .bind(&[KeyChord::new(KeyCode::Char('V'), KeyModifiers::SHIFT)], Cmd::EnterVisualLineMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())], Cmd::EnterSearchMode);

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
