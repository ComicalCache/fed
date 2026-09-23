use crate::types::Direction;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Move(Direction),
    Backspace,
    Delete,
    Escape,
    Input(String),
}
