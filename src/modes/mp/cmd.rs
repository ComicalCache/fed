use crate::types::Motion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Input(String),
    Move(Motion),
    Backspace,
    Delete,
    Submit,
    Close,
}
