use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Move(Motion),
    Delete,
    Change,
    Yank,
    SwapLineDown,
    SwapLineUp,
    EnterSearchMode,
    Escape,
}
