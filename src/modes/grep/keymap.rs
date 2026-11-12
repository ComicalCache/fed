use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::grep::cmd::Cmd;
use crate::modes::motions::MOTIONS;
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

    for (ch, motion) in
        [('H', Motion::Left), ('J', Motion::Down), ('K', Motion::Up), ('L', Motion::Right)]
    {
        keymap.bind(
            &[KeyChord::new(KeyCode::Char(ch), KeyModifiers::SHIFT)],
            Cmd::ScrollView(motion),
        );
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('n'), KeyModifiers::empty())], Cmd::NextMatch);
    keymap.bind(&[KeyChord::new(KeyCode::Char('N'), KeyModifiers::SHIFT)], Cmd::PrevMatch);

    keymap.bind(&[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())], Cmd::Replace);
    keymap.bind(&[KeyChord::new(KeyCode::Char('R'), KeyModifiers::SHIFT)], Cmd::ReplaceAll);

    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::CONTROL)], Cmd::FindFile);
    keymap.bind(&[KeyChord::new(KeyCode::Char('t'), KeyModifiers::CONTROL)], Cmd::SelectTheme);

    keymap.bind(&[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)], Cmd::Jump);

    keymap.bind(&[KeyChord::new(KeyCode::Enter, KeyModifiers::NONE)], Cmd::Select);

    for (ch, motion) in
        [('h', Motion::Left), ('j', Motion::Down), ('k', Motion::Up), ('l', Motion::Right)]
    {
        keymap.bind(
            &[
                KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                KeyChord::new(KeyCode::Char(ch), KeyModifiers::empty()),
            ],
            Cmd::FocusWindow(motion),
        );
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('d'), KeyModifiers::ALT)], Cmd::EnterDirMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('b'), KeyModifiers::ALT)], Cmd::EnterManageMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())], Cmd::EnterVisualMode);
    keymap
        .bind(&[KeyChord::new(KeyCode::Char('V'), KeyModifiers::SHIFT)], Cmd::EnterVisualLineMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())], Cmd::EnterSearchMode);
    keymap.bind(&[KeyChord::new(KeyCode::Char('F'), KeyModifiers::SHIFT)], Cmd::EnterGrepMode);

    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Cmd::Quit);

    keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Cmd::Close);

    keymap
}
