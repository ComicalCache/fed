use crate::{
    input::InputRouter,
    protocols::{cursor::CursorProtocol, screen::ScreenProtocol, view::ViewProtocol},
};

/// A struct containing all application state.
pub struct Fed {
    input_router: InputRouter,
    view: ViewProtocol,
    cursor: CursorProtocol,
    screen: ScreenProtocol,
}

impl Fed {
    pub fn new(
        input_router: InputRouter, view: ViewProtocol, cursor: CursorProtocol,
        screen: ScreenProtocol,
    ) -> Self {
        Self { input_router, view, cursor, screen }
    }

    /// Runs applications main event loop.
    pub async fn run(&mut self) {
        tokio::join!(
            self.input_router.run(),
            self.view.run(),
            self.cursor.run(),
            self.screen.run()
        );
    }
}
