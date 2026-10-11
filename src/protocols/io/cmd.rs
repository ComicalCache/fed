use std::{path::PathBuf, pin::Pin};

pub type IoFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type RCallback = Box<dyn FnOnce(Result<String, String>) -> IoFuture + Send>;
pub type WCallback = Box<dyn FnOnce(Result<(), String>) -> IoFuture + Send>;

pub enum IoCmd {
    Read { path: PathBuf, rcallback: RCallback },
    Write { path: PathBuf, data: String, wcallback: WCallback },
}
