use crate::types::Direction;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Move(Direction),
    Backspace,
    Delete,
    Submit,
    Close,
    Input(String),
}
