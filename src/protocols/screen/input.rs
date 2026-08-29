use crate::{
    input::InputHandler,
    protocols::{ProtocolInputPriority, screen::ScreenCommand},
    state::State,
};

pub struct ScreenInput {
    state: State,

    tx: flume::Sender<ScreenCommand>,
}

impl ScreenInput {
    pub fn new(state: State, tx: flume::Sender<ScreenCommand>) -> Self { Self { state, tx } }
}

impl InputHandler for ScreenInput {
    fn priority(&self) -> usize { ProtocolInputPriority::Screen as usize }

    fn resize(&mut self, size: (u16, u16)) {
        let (width, height) = (size.0 as usize, size.1 as usize);

        self.state.workspace.write().unwrap().resize(width, height);
        let _ = self.tx.send(ScreenCommand::Resize(width, height));
    }
}
