use tokio::sync::mpsc::UnboundedReceiver;

use crate::{render::Screen, state::StateLock};

pub enum ScreenCommand {
    Resize(usize, usize),
    Render,
}

pub struct ScreenProtocol {
    screen: Screen,

    state_lock: StateLock,

    rx: UnboundedReceiver<ScreenCommand>,
}

impl ScreenProtocol {
    pub fn new(
        state_lock: StateLock, width: usize, height: usize, rx: UnboundedReceiver<ScreenCommand>,
    ) -> Self {
        Self { screen: Screen::new(width, height), state_lock, rx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ScreenCommand::Resize(width, height) => self.screen.resize(width, height),
                ScreenCommand::Render => {
                    while matches!(self.rx.try_recv(), Ok(ScreenCommand::Render)) {}
                }
            }

            let state = self.state_lock.read();

            state.workspace.render(&state, &mut self.screen);

            drop(state);

            self.screen.render();
        }
    }
}
