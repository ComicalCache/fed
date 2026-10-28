use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
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
