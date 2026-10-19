use crate::{
    fed::FCmd,
    protocols::picker::PickerCmd,
    state::State,
    types::{ResizeInputHandler, ResizeInputPriority},
};

pub struct PickerResizeInput {}

impl ResizeInputHandler for PickerResizeInput {
    fn priority(&self) -> ResizeInputPriority { ResizeInputPriority::PickerProtocol }

    fn resize(&mut self, _: &State, (width, height): (u16, u16)) -> Vec<FCmd> {
        vec![FCmd::Picker(PickerCmd::Resize { width: width as usize, height: height as usize })]
    }
}
