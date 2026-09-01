use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{ResizeInputHandler, priorities::ResizeInputPriority},
    protocols::buffer::BufferCommand,
};

pub struct BufferResizeInput {
    tx: UnboundedSender<BufferCommand>,
}

impl BufferResizeInput {
    pub fn new(tx: UnboundedSender<BufferCommand>) -> Self { Self { tx } }
}

impl ResizeInputHandler for BufferResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::BufferProtocol }

    fn resize(&mut self, _: (u16, u16)) { let _ = self.tx.send(BufferCommand::Resize); }
}
