use crate::protocols::{io::IoPState, mp::MpPState, quit::QuitPState, screen::ScreenPState};

pub struct PState {
    pub io: IoPState,
    pub mp: MpPState,
    pub quit: QuitPState,
    pub screen: ScreenPState,
}

impl PState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            io: IoPState::default(),
            mp: MpPState::new(width, height),
            quit: QuitPState::default(),
            screen: ScreenPState::new(width, height),
        }
    }
}
