mod kind;

pub mod types {
    pub use crate::state::mini_buffer::kind::Kind;
}

use tokio::sync::oneshot;

use crate::{
    newtype::newtype,
    render::WindowId,
    state::{DocId, ViewId},
};

newtype!(MiniBufferId, usize);

#[derive(Default)]
pub struct MiniBuffer {
    pub id: MiniBufferId,
    pub kind: types::Kind,

    pub window: Option<WindowId>,
    pub prev_window: Option<WindowId>,

    pub doc: DocId,
    pub view: ViewId,

    pub res_tx: Option<oneshot::Sender<String>>,
    pub confirm_tx: Option<oneshot::Sender<bool>>,
}
