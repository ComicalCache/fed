use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    ScrollView(Motion),
    Delete,
    Change,
    Yank,
    Paste,
    SwapLineDown,
    SwapLineUp,
    Indent,
    Dedent,
    EnterVisualMode,
    EnterVisualLineMode,
    EnterSearchMode,
    SaveFile,
    FindFile,
    SelectTheme,
    Jump,
    FocusWindow(Motion),
    EnterDirMode,
    EnterManageMode,
    Quit,
    Escape,
}
