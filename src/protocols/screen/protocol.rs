use std::time::Duration;

use tokio::sync::mpsc::UnboundedReceiver;

use crate::{render::Screen, state::State};

pub enum ScreenCommand {
    Resize(usize, usize),
}

pub struct ScreenProtocol {
    screen: Screen,
    state: State,

    rx: UnboundedReceiver<ScreenCommand>,
}

impl ScreenProtocol {
    pub fn new(
        state: State, width: usize, height: usize, rx: UnboundedReceiver<ScreenCommand>,
    ) -> Self {
        Self { screen: Screen::new(width, height), state, rx }
    }

    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(Duration::from_millis(16));

        loop {
            tokio::select! {
                res = self.rx.recv() => {
                    let Some(res) = res else { break; };

                    match res {
                        ScreenCommand::Resize(width, height) => {
                            self.screen.resize(width , height);
                        }
                    }
                }

                _ = interval.tick() => {
                    self.state.with_workspace(|w| w.render(&mut self.screen));

                    self.screen.render();
                }
            }
        }
    }
}
