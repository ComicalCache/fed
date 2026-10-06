use crate::types::Motion;

#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    Jump,
    Select,
    EnterVisualMode,
    EnterSearchMode,
    Quit,
}
