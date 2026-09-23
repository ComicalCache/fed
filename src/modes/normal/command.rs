use crate::types::Direction;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Move(Direction),
    EnterInsertMode,
    Quit,
    TestMessage,
    TestPrompt,
}
