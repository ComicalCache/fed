use crate::{
    fed::FCmd,
    input_handler::{ResizeInputHandler, ResizeInputPriority},
    protocols::mp::MpCmd,
    state::State,
};

pub struct MpResizeInput {}

impl ResizeInputHandler for MpResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::MpProtocol }

    fn resize(&mut self, _: &State, (width, height): (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::Mp(MpCmd::Resize { width: width as usize, height: height as usize })]
    }
}
