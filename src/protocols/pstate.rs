use crate::protocols::{
    dir::DirPState, doc_view::DocViewPState, io::IoPState, manage::ManagePState, mp::MpPState,
    picker::PickerPState, quit::QuitPState,
};

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
