use crate::{state::ViewId, types::Pos};

pub enum ViewCmd {
    Update { view: ViewId },
    ScrollTo { view: ViewId, pos: Pos },

    Resize,
}
