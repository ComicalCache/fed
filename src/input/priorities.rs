#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyInputPriority {
    NormalMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseInputPriority {
    NormalMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum PasteInputPriority {}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ResizeInputPriority {
    BufferProtocol,
    // Needs the highest priority.
    ScreenProtocol,
}
