use std::{path::PathBuf, pin::Pin};

use crate::protocols::io::ScanEntry;

pub type IoFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type RCallback = Box<dyn FnOnce(Result<String, String>) -> IoFuture + Send>;
pub type WCallback = Box<dyn FnOnce(Result<(), String>) -> IoFuture + Send>;

#[allow(unused)]
pub enum IoCmd {
    Read { path: PathBuf, rcallback: RCallback },
    Write { path: PathBuf, data: String, wcallback: WCallback },
    Scan { path: PathBuf },
    ScanCompletion { path: PathBuf, revision: u64, entries: Vec<ScanEntry> },
}
