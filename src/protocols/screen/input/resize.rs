use crate::{
    fed::FCmd,
    input_handler::{ResizeInputHandler, ResizeInputPriority},
    protocols::screen::ScreenCmd,
    state::State,
};

pub struct ScreenResizeInput {}

impl ResizeInputHandler for ScreenResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ScreenProtocol }

    fn resize(&mut self, _: &State, (width, height): (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::Screen(ScreenCmd::Resize(width as usize, height as usize))]
    }
}
