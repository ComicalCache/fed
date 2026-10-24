use crate::protocols::action::ActionProtocol;
use crate::protocols::io::IoProtocol;
use crate::state::State;

pub type QuitCallback = fn(&State) -> Result<(), String>;

pub struct QuitPState {
    pub quit: bool,
    pub callbacks: &'static [QuitCallback],
}

impl Default for QuitPState {
    fn default() -> Self {
        Self { quit: false, callbacks: &[IoProtocol::can_quit, ActionProtocol::can_quit] }
    }
}
