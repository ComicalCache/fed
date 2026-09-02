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
    ViewProtocol,
    // Needs the highest priority.
    ScreenProtocol,
}
