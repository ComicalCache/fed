use std::path::PathBuf;
use std::pin::Pin;

use crate::protocols::io::IoFileEntry;
use crate::state::State;

pub type IoFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type RCallback = Box<dyn FnOnce(Result<String, String>) -> IoFuture + Send>;
pub type WCallback = Box<dyn FnOnce(Result<(), String>, u64) -> IoFuture + Send>;
pub type SCallback = Box<dyn FnOnce(&mut State, Vec<IoFileEntry>) + Send>;

#[allow(unused)]
pub enum IoCmd {
    Read { path: PathBuf, rcallback: RCallback },
    Write { path: PathBuf, data: String, wcallback: WCallback, revision: u64 },
    Completion { path: PathBuf },
    Scan { path: PathBuf, scallback: SCallback },
    ScanCompletion { entries: Vec<IoFileEntry>, scallback: SCallback, revision: u64 },
}
