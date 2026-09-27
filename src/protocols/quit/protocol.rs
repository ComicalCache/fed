use std::time::Duration;

use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::protocols::mini_buffer::MiniBufferCommand;

type QuitCallback = Box<dyn Send + Sync + Fn() -> oneshot::Receiver<Result<(), String>>>;

pub struct QuitProtocol {
    callbacks: Vec<QuitCallback>,

    rx: UnboundedReceiver<()>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    shutdown_tx: UnboundedSender<()>,
}

impl QuitProtocol {
    pub fn new(
        rx: UnboundedReceiver<()>, mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
        shutdown_tx: UnboundedSender<()>,
    ) -> Self {
        Self { callbacks: Vec::new(), rx, mini_buffer_tx, shutdown_tx }
    }

    pub fn add_callback(&mut self, cb: QuitCallback) { self.callbacks.push(cb); }

    pub async fn run(&mut self) {
        while let Some(_) = self.rx.recv().await {
            let mut can_quit = true;

            for callback in &self.callbacks {
                match callback().await {
                    Ok(Ok(())) => continue,
                    Ok(Err(message)) => {
                        can_quit = false;

                        let (tx, rx) = oneshot::channel();
                        let _ =
                            self.mini_buffer_tx.send(MiniBufferCommand::Message { message, tx });

                        let tx = self.mini_buffer_tx.clone();
                        tokio::spawn(async move {
                            let Ok(id) = rx.await else { return };
                            tokio::time::sleep(Duration::from_secs(3)).await;

                            let _ = tx.send(MiniBufferCommand::Close { id });
                        });

                        break;
                    }
                    Err(_) => continue,
                }
            }

            if can_quit {
                let _ = self.shutdown_tx.send(());
                break;
            }
        }
    }
}
