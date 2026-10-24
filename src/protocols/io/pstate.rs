use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use crate::protocols::io::ScanEntry;

#[derive(Default)]
pub struct IoPState {
    pub writers: Arc<AtomicUsize>,

    pub scan_list: Vec<ScanEntry>,
    pub scan_root: Option<PathBuf>,
    pub scan_revision: u64,
}
