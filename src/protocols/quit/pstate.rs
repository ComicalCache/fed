use crate::{
    protocols::{action::ActionProtocol, io::IoProtocol, pstate::PState},
    state::State,
};

pub type QuitCallback = fn(&State, &PState) -> Result<(), String>;

pub struct QuitPState {
    pub quit: bool,
    pub callbacks: &'static [QuitCallback],
}

impl Default for QuitPState {
    fn default() -> Self {
        Self { quit: false, callbacks: &[IoProtocol::can_quit, ActionProtocol::can_quit] }
    }
}
