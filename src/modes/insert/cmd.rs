use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Cmd {
    Input(String),
    Move(Motion),
    Backspace,
    Delete,
    Escape,
}
