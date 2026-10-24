use std::path::PathBuf;

use tokio::sync::oneshot;

use crate::state::DocId;
use crate::state::ViewId;
use crate::types::Rect;
use crate::types::RectSplit;
use crate::types::render::WindowId;
use crate::types::render::ZLayer;

#[allow(unused)]
pub enum DocViewCmd {
    OpenFile {
        window: WindowId,
        path: PathBuf,
        force: bool,
    },
    OpenFileCompletion {
        window: WindowId,
        id: u64,
        path: PathBuf,
        force: bool,
        res: Result<String, String>,
    },
    CreateTile {
        doc: DocId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
        raw: bool,
        tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
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
