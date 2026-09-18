// Priorities are sorted in reverse! Like on a downwards growing stack, the
// later the entry in the enum, the earlier it is checked and has a chance to
// intercept input events.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyInputPriority {
    NormalMode,
    InsertMode,

    // Needs the highest priority for intercepting with active mini buffer.
    MiniBufferMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseInputPriority {
    NormalMode,
    InsertMode,

    // Needs the highest priority for intercepting with active mini buffer.
    MiniBufferMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum PasteInputPriority {}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ResizeInputPriority {
    ViewProtocol,
    MiniBufferProtocol,

    // Needs the highest priority for resizing the screen buffer.
    ScreenProtocol,
}
