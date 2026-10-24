use tokio::sync::oneshot;

use crate::protocols::mp::MpId;
use crate::protocols::mp::PCallback;

#[allow(unused)]
pub enum MpCmd {
    Message {
        message: String,
        tx: Option<oneshot::Sender<MpId>>,
    },
    Prompt {
        prompt: String,
        initial_text: Option<String>,
        pcallback: PCallback,
        tx: Option<oneshot::Sender<MpId>>,
    },
    Submit,
    Close {
        id: MpId,
    },

    Resize {
        width: usize,
        height: usize,
    },
}
