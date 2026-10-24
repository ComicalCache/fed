use crate::fed::FCmd;
use crate::protocols::view::ViewCmd;
use crate::state::State;
use crate::types::ResizeInputHandler;
use crate::types::ResizeInputPriority;

pub struct ViewResizeInput {}

impl ResizeInputHandler for ViewResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ViewProtocol }

    fn resize(&mut self, _: &State, _: (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::View(ViewCmd::Resize)]
    }
}
