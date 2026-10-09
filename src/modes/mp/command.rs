use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Input(String),
    Move(Motion),
    Backspace,
    Delete,
    Submit,
    Close,
}
