use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;

use crate::protocols::io::cmd::RCallback;
use crate::protocols::io::cmd::WCallback;

#[derive(Clone)]
pub struct IoFileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub dir: bool,
}

pub enum IoRequest {
    Read { rcallback: RCallback },
    Write { data: String, wcallback: WCallback, revision: u64 },
}

#[derive(Default)]
pub struct IoPState {
    pub queues: HashMap<PathBuf, VecDeque<IoRequest>>,

    pub scan_revision: u64,
}
