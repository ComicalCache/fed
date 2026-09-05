use crate::{
    input::InputRouter,
    protocols::{
        action::ActionProtocol, io::IoProtocol, screen::ScreenProtocol, view::ViewProtocol,
    },
};

/// A struct containing all application state.
pub struct Fed {
    input_router: InputRouter,

    io: IoProtocol,
    view: ViewProtocol,
    action: ActionProtocol,
    screen: ScreenProtocol,
}

impl Fed {
    pub fn new(
        input_router: InputRouter, io: IoProtocol, view: ViewProtocol, action: ActionProtocol,
        screen: ScreenProtocol,
    ) -> Self {
        Self { input_router, io, view, action, screen }
    }

    /// Runs applications main event loop.
    pub async fn run(&mut self) {
        tokio::join!(
            self.input_router.run(),
            self.io.run(),
            self.view.run(),
            self.action.run(),
            self.screen.run()
        );
    }
}
