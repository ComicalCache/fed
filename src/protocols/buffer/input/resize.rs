use crate::{
    input::{ResizeInputHandler, priorities::ResizeInputPriority},
    protocols::buffer::BufferCommand,
};

pub struct BufferResizeInput {
    tx: flume::Sender<BufferCommand>,
}

impl BufferResizeInput {
    pub fn new(tx: flume::Sender<BufferCommand>) -> Self { Self { tx } }
}

impl ResizeInputHandler for BufferResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::BufferProtocol }

    fn resize(&mut self, _: (u16, u16)) { let _ = self.tx.send(BufferCommand::Resize); }
}
