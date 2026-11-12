use crate::types::Motion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    NextMatch,
    PrevMatch,
    Replace,
    ReplaceAll,
    FindFile,
    SelectTheme,
    Jump,
    Select,
    FocusWindow(Motion),
    EnterDirMode,
    EnterManageMode,
    EnterVisualMode,
    EnterVisualLineMode,
    EnterSearchMode,
    EnterGrepMode,
    Quit,
}
