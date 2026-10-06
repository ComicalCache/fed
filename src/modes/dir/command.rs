use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
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
    EnterVisualMode,
    EnterSearchMode,
    Quit,
}
