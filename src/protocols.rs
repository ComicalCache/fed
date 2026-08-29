pub mod buffer;
pub mod cursor;
pub mod screen;

enum ProtocolInputPriority {
    Buffer = 0,
    // Needs the highest priority for resize events.
    Screen,
}
