use crate::types::{Motion, RectSplit};

#[derive(Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    Jump,
    SplitWindow(RectSplit),
    FocusWindow(Motion),
    EnterManageMode,
    EnterDirMode,
    EnterVisualMode,
    EnterSearchMode,
    Quit,
}
