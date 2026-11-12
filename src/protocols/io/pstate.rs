use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

#[derive(Clone)]
pub struct IoFileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub dir: bool,
}

#[derive(Default)]
pub struct IoPState {
    pub writers: Arc<AtomicUsize>,

    pub scan_list: Vec<IoFileEntry>,
    pub scan_revision: u64,
}
