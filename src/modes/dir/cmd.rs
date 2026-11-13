use crate::types::Motion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    FindFile,
    SelectTheme,
    Jump,
    Create,
    Rename,
    Delete,
    DeleteRecursive,
    Select,
    FocusWindow(Motion),
    IncreaseWindow,
    DecreaseWindow,
    EnterManageMode,
    ShowKeybinds,
    EnterVisualMode,
    EnterVisualLineMode,
    EnterSearchMode,
    ShowGrep,
    EnterGrepMode,
    Quit,
}
