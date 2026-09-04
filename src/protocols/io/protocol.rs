use std::path::PathBuf;

use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

pub enum IoCommand {
    Read { path: PathBuf, tx: oneshot::Sender<Result<String, String>> },
    Write { path: PathBuf, data: String, tx: oneshot::Sender<Result<(), String>> },
}

pub struct IoProtocol {
    rx: UnboundedReceiver<IoCommand>,
}

impl IoProtocol {
    pub fn new(rx: UnboundedReceiver<IoCommand>) -> Self { Self { rx } }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                IoCommand::Read { path, tx } => self.read(path, tx).await,
                IoCommand::Write { path, data, tx } => self.write(path, data, tx).await,
            }
        }
    }

    async fn read(&mut self, path: PathBuf, tx: oneshot::Sender<Result<String, String>>) {
        tokio::spawn(async move {
            // TODO: chunked reading.
            let res = tokio::fs::read_to_string(&path).await.map_err(|err| err.to_string());

            let _ = tx.send(res);
        });
    }

    async fn write(
        &mut self, path: PathBuf, data: String, tx: oneshot::Sender<Result<(), String>>,
    ) {
        tokio::spawn(async move {
            let res = tokio::fs::write(&path, data).await.map_err(|err| err.to_string());

            let _ = tx.send(res);
        });
    }
}
