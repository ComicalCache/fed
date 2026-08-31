use crate::{
    input::{ResizeInputHandler, priorities::ResizeInputPriority},
    protocols::screen::ScreenCommand,
    state::State,
};

pub struct ScreenResizeInput {
    state: State,

    tx: flume::Sender<ScreenCommand>,
}

impl ScreenResizeInput {
    pub fn new(state: State, tx: flume::Sender<ScreenCommand>) -> Self { Self { state, tx } }
}

impl ResizeInputHandler for ScreenResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ScreenProtocol }

    fn resize(&mut self, size: (u16, u16)) {
        let (width, height) = (size.0 as usize, size.1 as usize);

        self.state.workspace.write().unwrap().resize(width, height);
        let _ = self.tx.send(ScreenCommand::Resize(width, height));
    }
}
