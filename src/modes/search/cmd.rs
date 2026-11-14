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
    SaveDoc,
    FindFile,
    SelectTheme,
    Jump,
    FocusWindow(Motion),
    IncreaseWindow,
    DecreaseWindow,
    EnterDirMode,
    EnterManageMode,
    ShowGrep,
    EnterGrepMode,
    Quit,
    Escape,
}
