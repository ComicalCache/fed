use crate::{state::ViewId, types::Pos};

pub enum ViewCommand {
    Update { view: ViewId },
    ScrollTo { view: ViewId, pos: Pos },

    Resize,
}
