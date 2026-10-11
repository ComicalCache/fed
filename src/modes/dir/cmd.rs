use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    Jump,
    Create,
    Rename,
    Delete,
    DeleteRecursive,
    Select,
    FocusWindow(Motion),
    EnterVisualMode,
    EnterSearchMode,
    Quit,
}
