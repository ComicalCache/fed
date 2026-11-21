use crate::types::Motion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Input(String),
    Tab,
    Move(Motion),
    Backspace,
    Delete,
    DeleteMotion(Motion),
    SaveDoc,
    FindFile,
    SelectDoc,
    SelectTheme,
    ToggleIndentation,
    Escape,
}
