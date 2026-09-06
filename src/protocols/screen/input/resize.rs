use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{ResizeInputHandler, priorities::ResizeInputPriority},
    protocols::screen::ScreenCommand,
    state::StateLock,
};

pub struct ScreenResizeInput {
    state_lock: StateLock,

    tx: UnboundedSender<ScreenCommand>,
}

impl ScreenResizeInput {
    pub fn new(state_lock: StateLock, tx: UnboundedSender<ScreenCommand>) -> Self {
        Self { state_lock, tx }
    }
}

impl ResizeInputHandler for ScreenResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ScreenProtocol }

    fn resize(&mut self, size: (u16, u16)) {
        let (width, height) = (size.0 as usize, size.1 as usize);

        let mut state = self.state_lock.write();

        state.workspace.resize(width, height);

        drop(state);

        let _ = self.tx.send(ScreenCommand::Resize(width, height));
    }
}
