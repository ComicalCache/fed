// Priorities are sorted in reverse! Like on a downwards growing stack, the
// later the entry in the enum, the earlier it is checked and has a chance to
// intercept input events.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyInputPriority {
    NormalMode,
    InsertMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseInputPriority {
    NormalMode,
    InsertMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum PasteInputPriority {}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ResizeInputPriority {
    ViewProtocol,
    // Needs the highest priority.
    ScreenProtocol,
}
