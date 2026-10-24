use crate::fed::FCmd;
use crate::protocols::mp::MpCmd;
use crate::state::State;
use crate::types::ResizeInputHandler;
use crate::types::ResizeInputPriority;

pub struct MpResizeInput {}

impl ResizeInputHandler for MpResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::MpProtocol }

    fn resize(&mut self, _: &State, (width, height): (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::Mp(MpCmd::Resize { width: width as usize, height: height as usize })]
    }
}
