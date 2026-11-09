use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use crate::protocols::io::FileEntry;

#[derive(Default)]
pub struct IoPState {
    pub writers: Arc<AtomicUsize>,

    pub scan_list: Vec<FileEntry>,
    pub scan_revision: u64,
}
