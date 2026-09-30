use crate::{
    render::WindowId,
    state::{DocId, ViewId},
    types::Pos,
};

pub enum ViewCommand {
    Init { window: WindowId, view: ViewId, doc: DocId },
    Update { view: ViewId },
    ScrollTo { view: ViewId, pos: Pos },

    Resize,
}
