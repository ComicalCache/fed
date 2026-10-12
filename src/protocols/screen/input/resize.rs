use crate::{
    fed::FCmd,
    protocols::screen::ScreenCmd,
    state::State,
    types::{ResizeInputHandler, ResizeInputPriority},
};

pub struct ScreenResizeInput {}

impl ResizeInputHandler for ScreenResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::ScreenProtocol }

    fn resize(&mut self, _: &State, (width, height): (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::Screen(ScreenCmd::Resize(width as usize, height as usize))]
    }
}
