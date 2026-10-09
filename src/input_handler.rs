use crossterm::event::{KeyEvent, MouseEvent};

use crate::{fed::FCmd, state::State};

/// A trait that should be implemented by protocol input handlers to receive
/// key input `Event`s.
pub trait KeyInputHandler {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> KeyInputPriority;

    /// Processes, or ignores, an input `Event::Key`. If the handler consumes
    /// the event, `Some` is to be returned, `None` otherwise.
    fn key(&mut self, _: &State, _: &KeyEvent) -> Option<Vec<FCmd>>;
}

/// A trait that should be implemented by protocol input handlers to receive
/// mouse input `Event`s.
pub trait MouseInputHandler {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> MouseInputPriority;

    /// Processes, or ignores, an input `Event::Mouse`. If the handler consumes
    /// the event, `Some` is to be returned, `None` otherwise.
    fn mouse(&mut self, _: &State, _: &MouseEvent) -> Option<Vec<FCmd>>;
}

/// A trait that should be implemented by protocol input handlers to receive
/// paste input `Event`s.
pub trait PasteInputHandler {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> PasteInputPriority;

    /// Processes, or ignores, an input `Event::Paste`. If the handler consumes
    /// the event, `Some` is to be returned, `None` otherwise.
    fn paste(&mut self, _: &State, _: &String) -> Option<Vec<FCmd>>;
}

/// A trait that should be implemented by protocol input handlers to receive
/// resize input `Event`s.
pub trait ResizeInputHandler {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> ResizeInputPriority;

    /// Processes, or ignores, an input `Event::Resize`.
    fn resize(&mut self, _: &State, _: (u16, u16)) -> Vec<FCmd>;
}

// The earlier the entry in the enum, the earlier it is checked and has a chance
// to intercept input events. The input handler order is enforced at startup.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyInputPriority {
    // Needs the highest priority for intercepting with active mini buffer.
    MiniBufferMode = 0,
    SearchMode,
    VisualMode,
    DirMode,
    InsertMode,
    NormalMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseInputPriority {
    // Needs the highest priority for intercepting with active mini buffer.
    MiniBufferMode = 0,
    SearchMode,
    VisualMode,
    DirMode,
    InsertMode,
    NormalMode,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum PasteInputPriority {}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ResizeInputPriority {
    // Needs the highest priority for resizing the screen buffer.
    ScreenProtocol = 0,
    MpProtocol,
    ViewProtocol,
}
