use crate::{
    input::InputHandler,
    protocols::{ProtocolInputPriority, buffer::BufferCommand},
};

pub struct BufferInput {
    tx: flume::Sender<BufferCommand>,
}

impl BufferInput {
    pub fn new(tx: flume::Sender<BufferCommand>) -> Self { Self { tx } }
}

impl InputHandler for BufferInput {
    fn priority(&self) -> usize { ProtocolInputPriority::Buffer as usize }

    fn resize(&mut self, _: (u16, u16)) { let _ = self.tx.send(BufferCommand::Resize); }
}
