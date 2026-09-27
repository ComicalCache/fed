use std::path::PathBuf;

use tokio::sync::oneshot;

pub enum IoCommand {
    Read { path: PathBuf, tx: oneshot::Sender<Result<String, String>> },
    Write { path: PathBuf, data: String, tx: oneshot::Sender<Result<(), String>> },

    CanQuit { tx: oneshot::Sender<Result<(), String>> },
}
