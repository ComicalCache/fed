use crate::protocols::{io::IoState, mp::MpState, quit::QuitState, screen::ScreenState};

pub struct PState {
    pub io: IoState,
    pub mp: MpState,
    pub quit: QuitState,
    pub screen: ScreenState,
}
