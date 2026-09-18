use crate::{
    input::InputRouter,
    protocols::{
        action::ActionProtocol, io::IoProtocol, mini_buffer::MiniBufferProtocol,
        screen::ScreenProtocol, view::ViewProtocol,
    },
};

pub struct Fed {
    input_router: InputRouter,

    action: ActionProtocol,
    io: IoProtocol,
    mini_buffer: MiniBufferProtocol,
    screen: ScreenProtocol,
    view: ViewProtocol,
}

impl Fed {
    pub fn new(
        input_router: InputRouter, action: ActionProtocol, io: IoProtocol,
        mini_buffer: MiniBufferProtocol, screen: ScreenProtocol, view: ViewProtocol,
    ) -> Self {
        Self { input_router, action, io, mini_buffer, screen, view }
    }

    pub async fn run(&mut self) {
        tokio::join!(
            self.input_router.run(),
            self.action.run(),
            self.io.run(),
            self.mini_buffer.run(),
            self.screen.run(),
            self.view.run(),
        );
    }
}
