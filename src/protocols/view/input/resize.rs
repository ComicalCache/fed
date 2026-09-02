use tokio::sync::mpsc::UnboundedSender;

use crate::{
    input::{ResizeInputHandler, priorities::ResizeInputPriority},
    protocols::view::ViewCommand,
};

pub struct ViewResizeInput {
    tx: UnboundedSender<ViewCommand>,
}

impl ViewResizeInput {
    pub fn new(tx: UnboundedSender<ViewCommand>) -> Self { Self { tx } }
}

impl ResizeInputHandler for ViewResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ViewProtocol }

    fn resize(&mut self, _: (u16, u16)) { let _ = self.tx.send(ViewCommand::Resize); }
}
