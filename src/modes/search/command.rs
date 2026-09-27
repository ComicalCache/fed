use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Move(Motion),
    NextMatch,
    PrevMatch,
    CursorsBegin,
    CursorsEnd,
    Replace,
    Escape,
}
