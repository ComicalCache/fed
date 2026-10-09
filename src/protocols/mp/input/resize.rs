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
        let (width, height) = (width as usize, height as usize);
        vec![FCmd::Mp(MpCmd::Resize { width, height })]
    }
}
