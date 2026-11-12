use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
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
    EnterManageMode,
    EnterVisualMode,
    EnterVisualLineMode,
    EnterSearchMode,
    EnterGrepMode,
    Quit,
}
