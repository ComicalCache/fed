use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use crate::modes::motions::MOTIONS;
use crate::modes::normal::cmd::Cmd;
use crate::types::KeyChord;
use crate::types::Keymap;
use crate::types::Motion;
use crate::types::RectSplit;

pub fn keymap() -> Keymap<Cmd> {
    let mut keymap = Keymap::new();

    for (chord, motion) in MOTIONS {
        keymap.bind(&[chord], Cmd::Move(motion), "Move the cursor with the motion");
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()), chord],
            Cmd::Delete(motion),
            "Delete the motion",
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()), chord],
            Cmd::Change(motion),
            "Change the motion",
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()), chord],
            Cmd::Yank(motion),
            "Yank the motion",
        );
    }
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('d'), KeyModifiers::empty()),
        ],
        Cmd::DeleteLine,
        "Delete the line",
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('c'), KeyModifiers::empty()),
        ],
        Cmd::ChangeLine,
        "Change the line",
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('y'), KeyModifiers::empty()),
        ],
        Cmd::YankLine,
        "Yank the line",
    );

    for (ch, motion) in
        [('H', Motion::Left), ('J', Motion::Down), ('K', Motion::Up), ('L', Motion::Right)]
    {
        keymap.bind(
            &[KeyChord::new(KeyCode::Char(ch), KeyModifiers::SHIFT)],
            Cmd::ScrollView(motion),
            "Scroll the view",
        );
    }

    keymap.bind(&[KeyChord::new(KeyCode::Char('u'), KeyModifiers::empty())], Cmd::Undo, "Undo");
    keymap.bind(&[KeyChord::new(KeyCode::Char('U'), KeyModifiers::SHIFT)], Cmd::HotRedo, "Redo");

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('a'), KeyModifiers::empty())],
        Cmd::Append,
        "Enter insert mode after the cursor",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('A'), KeyModifiers::SHIFT)],
        Cmd::AppendEndOfLine,
        "Enter insert mode at the end of the line",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('o'), KeyModifiers::empty())],
        Cmd::InsertLineBelow,
        "Enter insert mode on a new line below",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('O'), KeyModifiers::SHIFT)],
        Cmd::InsertLineAbove,
        "Enter insert mode on a new line above",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('j'), KeyModifiers::ALT)],
        Cmd::SwapLineDown,
        "Swap two lines downwards",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('k'), KeyModifiers::ALT)],
        Cmd::SwapLineUp,
        "Swipe lines upwards",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())],
        Cmd::Indent,
        "Indent the lines",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::BackTab, KeyModifiers::SHIFT)],
        Cmd::Dedent,
        "Dedent the lines",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('p'), KeyModifiers::empty())],
        Cmd::Paste,
        "Paste the clipboard at the cursor",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('x'), KeyModifiers::empty())],
        Cmd::DeleteChar,
        "Delete the character under the cursor",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('r'), KeyModifiers::empty())],
        Cmd::Replace,
        "Replace the character under the cursor",
    );

    keymap.bind(
        &[KeyChord::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
        Cmd::SaveFile,
        "Save the file",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('x'), KeyModifiers::CONTROL)],
        Cmd::DiscardChanges,
        "Discard unsaved changes",
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

    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('H'), KeyModifiers::SHIFT),
        ],
        Cmd::SplitWindow(RectSplit::Horizontal),
        "Split the window",
    );
    keymap.bind(
        &[
            KeyChord::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('V'), KeyModifiers::SHIFT),
        ],
        Cmd::SplitWindow(RectSplit::Vertical),
        "Split the window",
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
        &[KeyChord::new(KeyCode::Char('i'), KeyModifiers::empty())],
        Cmd::EnterInsertMode,
        "Enter insert mode",
    );
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
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('h'), KeyModifiers::ALT)],
        Cmd::ShowKeybinds,
        "Open the keybind viewer",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('v'), KeyModifiers::empty())],
        Cmd::EnterVisualMode,
        "Start a selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('V'), KeyModifiers::SHIFT)],
        Cmd::EnterVisualLineMode,
        "Start a linewise selection",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('f'), KeyModifiers::empty())],
        Cmd::EnterSearchMode,
        "Search the current document",
    );
    keymap.bind(
        &[KeyChord::new(KeyCode::Char('F'), KeyModifiers::SHIFT)],
        Cmd::EnterGrepMode,
        "Search the current working directory",
    );

    keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Cmd::Quit, "Quit fed");

    keymap
}
