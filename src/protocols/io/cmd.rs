use std::{fmt, path::PathBuf, pin::Pin};

pub type IoFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type RCallback = Box<dyn FnOnce(Result<String, String>) -> IoFuture + Send>;
pub type WCallback = Box<dyn FnOnce(Result<(), String>) -> IoFuture + Send>;

#[derive(Clone)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub dir: bool,
}

impl fmt::Display for ScanEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.path.display(), if self.dir { "/" } else { "" })
    }
}

#[allow(unused)]
pub enum IoCmd {
    Read { path: PathBuf, rcallback: RCallback },
    Write { path: PathBuf, data: String, wcallback: WCallback },
    Scan { path: PathBuf },
    ScanCompletion { path: PathBuf, revision: u64, entries: Vec<ScanEntry> },
}
