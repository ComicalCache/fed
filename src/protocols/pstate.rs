use crate::protocols::{
    dir::DirPState, io::IoPState, manage::ManagePState, mp::MpPState, quit::QuitPState,
};

pub struct PState {
    pub io: IoPState,
    pub mp: MpPState,
    pub quit: QuitPState,
    pub manage: ManagePState,
    pub dir: DirPState,
}

impl PState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            io: IoPState::default(),
            mp: MpPState::new(width, height),
            quit: QuitPState::default(),
            manage: ManagePState::default(),
            dir: DirPState::default(),
        }
    }
}
