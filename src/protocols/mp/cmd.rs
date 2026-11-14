use tokio::sync::oneshot;

use crate::protocols::mp::MpId;
use crate::protocols::mp::PCallback;
use crate::protocols::mp::PromptHistory;

#[allow(unused)]
pub enum MpCmd {
    Message {
        message: String,
        tx: Option<oneshot::Sender<MpId>>,
    },
    Prompt {
        prompt: String,
        initial_text: Option<String>,
        history: Option<PromptHistory>,
        pcallback: PCallback,
        tx: Option<oneshot::Sender<MpId>>,
    },
    History {
        forward: bool,
    },
    Submit,
    Close {
        mp: MpId,
    },

    Resize {
        width: usize,
        height: usize,
    },
}
