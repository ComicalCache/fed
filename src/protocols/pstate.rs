use crate::protocols::dir::DirPState;
use crate::protocols::doc_view::DocViewPState;
use crate::protocols::io::IoPState;
use crate::protocols::manage::ManagePState;
use crate::protocols::mp::MpPState;
use crate::protocols::picker::PickerPState;
use crate::protocols::quit::QuitPState;

pub struct PState {
    pub io: IoPState,
    pub mp: MpPState,
    pub quit: QuitPState,
    pub manage: ManagePState,
    pub dir: DirPState,
    pub doc_view: DocViewPState,
    pub picker: PickerPState,
}

impl PState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            io: IoPState::default(),
            mp: MpPState::new(width, height),
            quit: QuitPState::default(),
            manage: ManagePState::default(),
            dir: DirPState::default(),
            doc_view: DocViewPState::default(),
            picker: PickerPState::default(),
        }
    }
}
