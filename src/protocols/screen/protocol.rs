use tokio::sync::mpsc::UnboundedReceiver;

use crate::{protocols::screen::ScreenCommand, render::Screen, state::StateLock};

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
        while let Some(mut cmd) = self.rx.recv().await {
            loop {
                match cmd {
                    ScreenCommand::Resize(width, height) => self.screen.resize(width, height),
                    ScreenCommand::Render => {}
                }

                // Debounce.
                let try_res = self.rx.try_recv();
                cmd = if try_res.is_ok() { try_res.unwrap() } else { break };
            }

            let state = self.state_lock.read();
            state.workspace.render(&state, &mut self.screen);
            drop(state);

            self.screen.render();
        }
    }
}
