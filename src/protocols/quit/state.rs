use crate::{
    protocols::{action::ActionProtocol, io::IoProtocol, state::PState},
    state::State,
};

pub type QuitCallback = fn(&State, &PState) -> Result<(), String>;

pub struct QuitState {
    pub quit: bool,
    pub callbacks: &'static [QuitCallback],
}

impl Default for QuitState {
    fn default() -> Self {
        Self { quit: false, callbacks: &[IoProtocol::can_quit, ActionProtocol::can_quit] }
    }
}
