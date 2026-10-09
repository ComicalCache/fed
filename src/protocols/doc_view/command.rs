use tokio::sync::oneshot;

use crate::{
    render::{WindowId, ZLayer},
    state::{DocId, ViewId},
    types::{Rect, RectSplit},
};

pub enum DocViewCmd {
    CreateTile {
        doc: DocId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
        raw: bool,
        tx: Option<oneshot::Sender<Option<(ViewId, WindowId)>>>,
    },
    CreateFloating {
        doc: DocId,
        view: Option<ViewId>,
        rect: Rect,
        z: ZLayer,
        raw: bool,
        tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    },
    ReplaceWindow {
        doc: DocId,
        view: Option<ViewId>,
        window: WindowId,
        raw: bool,
        tx: Option<oneshot::Sender<ViewId>>,
    },
    DestroyView {
        view: ViewId,
    },
}
