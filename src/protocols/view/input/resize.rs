use crate::{
    fed::FCmd,
    input_handler::{ResizeInputHandler, ResizeInputPriority},
    protocols::view::ViewCmd,
    state::State,
};

pub struct ViewResizeInput {}

impl ResizeInputHandler for ViewResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ViewProtocol }

    fn resize(&mut self, _: &State, _: (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::View(ViewCmd::Resize)]
    }
}
