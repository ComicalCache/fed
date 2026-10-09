mod kind;

pub mod types {
    pub use crate::state::mp::kind::Kind;
}

use crate::{
    newtype::newtype,
    render::WindowId,
    state::{DocId, ViewId},
};

newtype!(MpId, usize);

#[derive(Default)]
pub struct Mp {
    pub id: MpId,
    pub kind: types::Kind,

    pub window: Option<WindowId>,
    pub prev_window: Option<WindowId>,

    pub doc: DocId,
    pub view: ViewId,
}
