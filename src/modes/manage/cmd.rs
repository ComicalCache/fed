use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    Jump,
    Select,
    Close,
    FocusWindow(Motion),
    EnterDirMode,
    EnterVisualMode,
    EnterSearchMode,
    Quit,
}
