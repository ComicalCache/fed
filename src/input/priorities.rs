// The earlier the entry in the enum, the earlier it is checked and has a chance
// to intercept input events. The input handler order is enforced at startup.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyInputPriority {
    // Needs the highest priority for intercepting with active mini buffer.
    MiniBufferMode = 0,
    SearchMode,
    VisualMode,
    InsertMode,
    NormalMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseInputPriority {
    // Needs the highest priority for intercepting with active mini buffer.
    MiniBufferMode = 0,
    SearchMode,
    VisualMode,
    InsertMode,
    NormalMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum PasteInputPriority {}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ResizeInputPriority {
    // Needs the highest priority for resizing the screen buffer.
    ScreenProtocol = 0,
    MiniBufferProtocol,
    ViewProtocol,
}
