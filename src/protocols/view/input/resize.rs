use crate::{
    fed::FCmd,
    protocols::view::ViewCmd,
    state::State,
    types::{ResizeInputHandler, ResizeInputPriority},
};

pub struct ViewResizeInput {}

impl ResizeInputHandler for ViewResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ViewProtocol }

    fn resize(&mut self, _: &State, _: (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::View(ViewCmd::Resize)]
    }
}
