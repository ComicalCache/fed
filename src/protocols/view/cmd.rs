use crate::state::ViewId;
use crate::types::Pos;

#[allow(unused)]
pub enum ViewCmd {
    Update { view: ViewId },
    ScrollTo { view: ViewId, pos: Pos },

    Resize,
}
