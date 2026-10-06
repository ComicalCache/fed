use crate::{
    input::InputRouter,
    protocols::{
        action::ActionProtocol, dir::DirProtocol, doc_view::DocViewProtocol, io::IoProtocol,
        mini_buffer::MiniBufferProtocol, quit::QuitProtocol, screen::ScreenProtocol,
        view::ViewProtocol,
    },
};

pub struct Fed {
    input_router: InputRouter,

    action: ActionProtocol,
    io: IoProtocol,
    mini_buffer: MiniBufferProtocol,
    quit: QuitProtocol,
    screen: ScreenProtocol,
    view: ViewProtocol,
    doc_view: DocViewProtocol,
    dir: DirProtocol,
}

impl Fed {
    pub fn new(
        input_router: InputRouter, action: ActionProtocol, io: IoProtocol,
        mini_buffer: MiniBufferProtocol, quit: QuitProtocol, screen: ScreenProtocol,
        view: ViewProtocol, doc_view: DocViewProtocol, dir: DirProtocol,
    ) -> Self {
        Self { input_router, action, io, mini_buffer, quit, screen, view, doc_view, dir }
    }

    pub async fn run(&mut self) {
        tokio::join!(
            self.input_router.run(),
            self.action.run(),
            self.io.run(),
            self.mini_buffer.run(),
            self.quit.run(),
            self.screen.run(),
            self.view.run(),
            self.doc_view.run(),
            self.dir.run(),
        );
    }
}
