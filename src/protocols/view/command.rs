use tokio::sync::oneshot;

use crate::{
    render::{WindowId, ZLayer},
    state::{DocId, ViewId},
    types::{Pos, Rect, RectSplit},
};

pub enum ViewCommand {
    Init {
        window: WindowId,
        view: ViewId,
        doc: DocId,
    },
    Update {
        view: ViewId,
    },
    ScrollTo {
        view: ViewId,
        pos: Pos,
    },

    CreateRawTile {
        doc: DocId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateTile {
        doc: DocId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateFloating {
        doc: DocId,
        view: Option<ViewId>,
        rect: Rect,
        z: ZLayer,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateRawFloating {
        doc: DocId,
        view: Option<ViewId>,
        rect: Rect,
        z: ZLayer,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    DestroyView {
        view: ViewId,
    },

    Resize,
}
