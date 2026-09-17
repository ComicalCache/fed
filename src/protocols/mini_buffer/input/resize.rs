use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{ResizeInputHandler, priorities::ResizeInputPriority},
    protocols::mini_buffer::MiniBufferCommand,
};

pub struct MiniBufferResizeInput {
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
}

impl MiniBufferResizeInput {
    pub fn new(mini_buffer_tx: UnboundedSender<MiniBufferCommand>) -> Self {
        Self { mini_buffer_tx }
    }
}

impl ResizeInputHandler for MiniBufferResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::MiniBufferProtocol }

    fn resize(&mut self, (width, height): (u16, u16)) {
        let (width, height) = (width as usize, height as usize);
        let _ = self.mini_buffer_tx.send(MiniBufferCommand::Resize { width, height });
    }
}
