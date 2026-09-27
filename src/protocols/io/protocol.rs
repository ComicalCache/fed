use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

use crate::protocols::io::IoCommand;

pub struct IoProtocol {
    writers: Arc<AtomicUsize>,

    rx: UnboundedReceiver<IoCommand>,
}

impl IoProtocol {
    pub fn new(rx: UnboundedReceiver<IoCommand>) -> Self {
        Self { writers: Arc::new(AtomicUsize::new(0)), rx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                IoCommand::Read { path, tx } => self.read(path, tx),
                IoCommand::Write { path, data, tx } => self.write(path, data, tx),

                IoCommand::CanQuit { tx } => self.can_quit(tx),
            }
        }
    }

    fn read(&self, path: PathBuf, tx: oneshot::Sender<Result<String, String>>) {
        tokio::spawn(async move {
            let _ = tx.send(tokio::fs::read_to_string(&path).await.map_err(|err| err.to_string()));
        });
    }

    fn write(&self, path: PathBuf, data: String, tx: oneshot::Sender<Result<(), String>>) {
        self.writers.fetch_add(1, Ordering::Relaxed);

        let writers = self.writers.clone();
        tokio::spawn(async move {
            let data = tokio::fs::write(&path, data).await.map_err(|err| err.to_string());
            writers.fetch_sub(1, Ordering::Relaxed);

            let _ = tx.send(data);
        });
    }

    fn can_quit(&self, tx: oneshot::Sender<Result<(), String>>) {
        let writes = self.writers.load(Ordering::Relaxed);
        if writes > 0 {
            let _ = tx.send(Err(format!("{writes} ongoing writes in progress")));
            return;
        }

        let _ = tx.send(Ok(()));
    }
}
