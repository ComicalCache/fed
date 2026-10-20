use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};

use crate::protocols::io::ScanEntry;

#[derive(Default)]
pub struct IoPState {
    pub writers: Arc<AtomicUsize>,

    pub scan_list: Vec<ScanEntry>,
    pub scan_root: Option<PathBuf>,
    pub scan_revision: u64,
}
