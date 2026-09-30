use tokio::sync::oneshot;

use crate::{
    render::{WindowId, ZLayer},
    state::{DocId, ViewId},
    types::{Rect, RectSplit},
};

pub enum DocViewCommand {
    CreateTile {
        doc: DocId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
        raw: bool,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateFloating {
        doc: DocId,
        view: Option<ViewId>,
        rect: Rect,
        z: ZLayer,
        raw: bool,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    DestroyView {
        view: ViewId,
    },
}
