use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Delete,
    Change,
    Yank,
    SwapLineDown,
    SwapLineUp,
    EnterSearchMode,
    Escape,
}
