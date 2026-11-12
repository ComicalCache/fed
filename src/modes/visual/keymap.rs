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
        keymap.bind(&[chord], Cmd::Move(motion), "Move the cursor with the motion");
    }

    for (ch, motion) in
        [('H', Motion::Left), ('J', Motion::Down), ('K', Motion::Up), ('L', Motion::Right)]
    {
        keymap.bind(
            &[KeyChord::new(KeyCode::Char(ch), KeyModifiers::SHIFT)],
            Cmd::ScrollView(motion),
            "Scroll the view",
        );
    }

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty())],
        Cmd::Delete,
        "Delete the selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty())],
        Cmd::Change,
        "Change the selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty())],
        Cmd::Yank,
        "Yank the selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('p'), KeyModifiers::empty())],
        Cmd::Paste,
        "Replace the selection with the clipboard",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('j'), KeyModifiers::ALT)],
        Cmd::SwapLineDown,
        "Swap the selected lines downwards",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('k'), KeyModifiers::ALT)],
        Cmd::SwapLineUp,
        "Swap the selected lines upwards",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())],
        Cmd::Indent,
        "Indent the selected lines",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::BackTab, KeyModifiers::SHIFT)],
        Cmd::Dedent,
        "Dedent the selected lines",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('a'), KeyModifiers::CONTROL)],
        Cmd::Align,
        "Align the selected lines' contents",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('a'), KeyModifiers::CONTROL | KeyModifiers::SHIFT)],
        Cmd::AlignRepeated,
        "Align the selected lines' contents repeatedly",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('o'), KeyModifiers::CONTROL)],
        Cmd::Sort,
        "Sort the selected lines alphabetically",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('o'), KeyModifiers::CONTROL | KeyModifiers::SHIFT)],
        Cmd::SortReverse,
        "Sort the selected lines alphabetically in reverse",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())],
        Cmd::EnterVisualMode,
        "Toggle the selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('V'), KeyModifiers::SHIFT)],
        Cmd::EnterVisualLineMode,
        "Toggle the linewise selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())],
        Cmd::EnterSearchMode,
        "Search the selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('F'), KeyModifiers::SHIFT)],
        Cmd::EnterGrepMode,
        "Search the current working directory",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
        Cmd::SaveFile,
        "Save the file",
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
        &[KeyChord::new(KeyCode::Char('j'), KeyModifiers::CONTROL)],
        Cmd::Jump,
        "Jump to a line",
    );

    for (ch, motion) in
        [('h', Motion::Left), ('j', Motion::Down), ('k', Motion::Up), ('l', Motion::Right)]
    {
        keymap.bind(
            &[
                KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
                KeyChord::new(KeyCode::Char(ch), KeyModifiers::empty()),
            ],
            Cmd::FocusWindow(motion),
            "Focus the window",
        );
    }

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('d'), KeyModifiers::ALT)],
        Cmd::EnterDirMode,
        "Open the directory viewer",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('b'), KeyModifiers::ALT)],
        Cmd::EnterManageMode,
        "Open the document and view manager",
    );
    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Cmd::Quit, "Quit fed");

    keymap.bind(
        &[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())],
        Cmd::Escape,
        "Exit visual mode",
    );

    keymap
}
