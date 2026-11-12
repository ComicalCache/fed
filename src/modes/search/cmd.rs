use crate::types::Motion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    NextMatch,
    PrevMatch,
    CursorsBegin,
    CursorsEnd,
    Replace,
    ReplaceAll,
    SaveFile,
    FindFile,
    SelectTheme,
    Jump,
    FocusWindow(Motion),
    EnterDirMode,
    EnterManageMode,
    EnterGrepMode,
    Quit,
    Escape,
}
