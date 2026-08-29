use crate::{
    input::InputRouter,
    protocols::{buffer::BufferProtocol, cursor::CursorProtocol, screen::ScreenProtocol},
};

/// A struct containing all application state.
pub struct Fed {
    input_router: InputRouter,
    buffer: BufferProtocol,
    cursor: CursorProtocol,
    screen: ScreenProtocol,
}

impl Fed {
    pub fn new(
        input_router: InputRouter, buffer: BufferProtocol, cursor: CursorProtocol,
        screen: ScreenProtocol,
    ) -> Self {
        Self { input_router, buffer, cursor, screen }
    }

    /// Runs applications main event loop.
    pub async fn run(&mut self) {
        tokio::join!(
            self.input_router.run(),
            self.buffer.run(),
            self.cursor.run(),
            self.screen.run()
        );
    }
}
