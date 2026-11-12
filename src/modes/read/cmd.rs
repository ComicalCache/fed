use crate::types::Motion;
use crate::types::RectSplit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Move(Motion),
    Yank(Motion),
    YankLine,
    ScrollView(Motion),
    FindFile,
    SelectTheme,
    Jump,
    SplitWindow(RectSplit),
    FocusWindow(Motion),
    EnterManageMode,
    EnterDirMode,
    ShowKeybinds,
    EnterVisualMode,
    EnterVisualLineMode,
    EnterSearchMode,
    EnterGrepMode,
    Quit,
}
