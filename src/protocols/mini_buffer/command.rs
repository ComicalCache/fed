use tokio::sync::oneshot;

use crate::state::MiniBufferId;

pub enum MiniBufferCommand {
    Message {
        message: String,
        tx: oneshot::Sender<MiniBufferId>,
    },
    Prompt {
        prompt: String,
        initial_text: Option<String>,
        id_tx: oneshot::Sender<MiniBufferId>,
        res_tx: oneshot::Sender<String>,
    },
    Submit,
    Close {
        id: MiniBufferId,
    },
    Resize {
        width: usize,
        height: usize,
    },
}
