use crossterm::event::{Event, KeyEvent, MouseEvent};

/// A trait that should be implemented by protocol input handlers to receive
/// input `Event`s.
pub trait InputHandler: Send + Sync + 'static {
    /// The _static_ priority of the handler. This determines in which order
    /// handlers are called.
    fn priority(&self) -> usize;

    /// Processes, or ignores, an input `Event::Key`. If the handler consumes
    /// the event, `true` is to be returned, `false` otherwise. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn key(&mut self, _: &KeyEvent) -> bool { false }

    /// Processes, or ignores, an input `Event::Mouse`. If the handler consumes
    /// the event, `true` is to be returned, `false` otherwise. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn mouse(&mut self, _: &MouseEvent) -> bool { false }

    /// Processes, or ignores, an input `Event::Paste`. If the handler consumes
    /// the event, `true` is to be returned, `false` otherwise. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn paste(&mut self, _: &String) -> bool { false }

    /// Processes, or ignores, an input `Event::Resize`. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    /// Consider creating a handler struct that communicates with the main
    /// protocol struct.
    fn resize(&mut self, _: (u16, u16)) {}
}

/// A router which routes input `Event`s from the front-end to the protocols.
pub struct InputRouter {
    handlers: Vec<Box<dyn InputHandler>>,
    rx: flume::Receiver<Event>,
}

impl InputRouter {
    pub fn new(mut handlers: Vec<Box<dyn InputHandler>>, rx: flume::Receiver<Event>) -> Self {
        handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));

        Self { handlers, rx }
    }

    /// Runs the input routers main event loop, routing input `Event`s to all
    /// input handlers.
    pub async fn run(&mut self) {
        while let Ok(event) = self.rx.recv_async().await {
            for handler in &mut self.handlers {
                match event {
                    Event::Key(key) if handler.key(&key) => break,
                    Event::Mouse(mouse) if handler.mouse(&mouse) => break,
                    Event::Paste(str) if handler.paste(&str) => break,
                    Event::Resize(width, height) => handler.resize((width, height)),
                    _ => {}
                }
            }
        }
    }
}
