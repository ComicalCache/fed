use crate::types::Motion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    FindFile,
    SelectTheme,
    Jump,
    Select,
    Close,
    FocusWindow(Motion),
    IncreaseWindow,
    DecreaseWindow,
    EnterDirMode,
    ShowKeybinds,
    EnterVisualMode,
    EnterVisualLineMode,
    EnterSearchMode,
    ShowGrep,
    EnterGrepMode,
    EnterGrepModeInDir,
    Quit,
}
