pub mod priorities;

use crossterm::event::{Event, KeyEvent, MouseEvent};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    input::priorities::{
        KeyInputPriority, MouseInputPriority, PasteInputPriority, ResizeInputPriority,
    },
    state::StateLock,
};

/// A trait that should be implemented by protocol input handlers to receive
/// key input `Event`s.
pub trait KeyInputHandler: Send + Sync + 'static {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> KeyInputPriority;

    /// Processes, or ignores, an input `Event::Key`. If the handler consumes
    /// the event, `true` is to be returned, `false` otherwise. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn key(&mut self, _: &KeyEvent) -> bool;
}

/// A trait that should be implemented by protocol input handlers to receive
/// mouse input `Event`s.
pub trait MouseInputHandler: Send + Sync + 'static {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> MouseInputPriority;

    /// Processes, or ignores, an input `Event::Mouse`. If the handler consumes
    /// the event, `true` is to be returned, `false` otherwise. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn mouse(&mut self, _: &MouseEvent) -> bool;
}

/// A trait that should be implemented by protocol input handlers to receive
/// paste input `Event`s.
pub trait PasteInputHandler: Send + Sync + 'static {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> PasteInputPriority;

    /// Processes, or ignores, an input `Event::Paste`. If the handler consumes
    /// the event, `true` is to be returned, `false` otherwise. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn paste(&mut self, _: &String) -> bool;
}

/// A trait that should be implemented by protocol input handlers to receive
/// resize input `Event`s.
pub trait ResizeInputHandler: Send + Sync + 'static {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> ResizeInputPriority;

    /// Processes, or ignores, an input `Event::Resize`. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn resize(&mut self, _: (u16, u16));
}

/// A router which routes input `Event`s from the front-end to the protocols.
pub struct InputRouter {
    key_handlers: Vec<Box<dyn KeyInputHandler>>,
    mouse_handlers: Vec<Box<dyn MouseInputHandler>>,
    paste_handlers: Vec<Box<dyn PasteInputHandler>>,
    resize_handlers: Vec<Box<dyn ResizeInputHandler>>,

    state_lock: StateLock,

    rx: UnboundedReceiver<Event>,
}

impl InputRouter {
    pub fn new(state_lock: StateLock, rx: UnboundedReceiver<Event>) -> Self {
        Self {
            key_handlers: Vec::new(),
            mouse_handlers: Vec::new(),
            paste_handlers: Vec::new(),
            resize_handlers: Vec::new(),
            state_lock,
            rx,
        }
    }

    pub fn add_key_handler(&mut self, handler: Box<dyn KeyInputHandler>) {
        self.key_handlers.push(handler);
    }

    pub fn add_mouse_handler(&mut self, handler: Box<dyn MouseInputHandler>) {
        self.mouse_handlers.push(handler);
    }

    pub fn add_paste_handler(&mut self, handler: Box<dyn PasteInputHandler>) {
        self.paste_handlers.push(handler);
    }

    pub fn add_resize_handler(&mut self, handler: Box<dyn ResizeInputHandler>) {
        self.resize_handlers.push(handler);
    }

    /// Runs the input routers main event loop, routing input `Event`s to all
    /// input handlers.
    pub async fn run(&mut self) {
        self.key_handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));
        self.mouse_handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));
        self.paste_handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));
        self.resize_handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));

        while let Some(event) = self.rx.recv().await {
            match event {
                Event::Key(key) => {
                    for handler in &mut self.key_handlers {
                        if handler.key(&key) {
                            break;
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    let mut state = self.state_lock.write();

                    if let Some(w) = state.workspace.get_window((mouse.column, mouse.row).into()) {
                        state.workspace.active_window = Some(w);
                    }

                    drop(state);

                    for handler in &mut self.mouse_handlers {
                        if handler.mouse(&mouse) {
                            break;
                        }
                    }
                }
                Event::Paste(str) => {
                    for handler in &mut self.paste_handlers {
                        if handler.paste(&str) {
                            break;
                        }
                    }
                }
                Event::Resize(width, height) => {
                    for handler in &mut self.resize_handlers {
                        handler.resize((width, height));
                    }
                }
                _ => {}
            }
        }
    }
}
