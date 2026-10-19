use tokio::sync::oneshot;

use crate::{
    protocols::{
        doc_view::{DocViewCmd, DocViewRenderer},
        view::ViewProtocol,
    },
    state::{DocId, State, ViewId},
    types::{
        Rect, RectSplit,
        render::{WindowId, ZLayer},
    },
};

pub struct DocViewProtocol {}

impl DocViewProtocol {
    pub fn exec(s: &mut State, cmd: DocViewCmd) {
        match cmd {
            DocViewCmd::CreateTile { doc, view, split_window, direction, raw, tx } => {
                Self::create_tile(s, doc, view, split_window, direction, raw, tx);
            }
            DocViewCmd::CreateFloating { doc, view, rect, z, raw, tx } => {
                Self::create_floating(s, doc, view, rect, z, raw, tx);
            }
            DocViewCmd::ReplaceWindow { doc, view, window, raw, tx } => {
                Self::replace_window(s, doc, view, window, raw, tx);
            }
            DocViewCmd::DestroyView { view } => Self::destroy_view(s, view),
        }
    }

    pub fn create_tile(
        s: &mut State, doc: DocId, view: Option<ViewId>, split_window: WindowId,
        direction: RectSplit, raw: bool, tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    ) -> (ViewId, WindowId) {
        let view = Self::prepare_view(s, doc, view, raw);
        let window = s.workspace.create_tile(
            split_window,
            direction,
            Box::new(DocViewRenderer::new(view, raw)),
        );
        s.attach_view(window, view);

        ViewProtocol::resize(s);

        if let Some(tx) = tx {
            let _ = tx.send((view, window));
        }

        (view, window)
    }

    pub fn create_floating(
        s: &mut State, doc: DocId, view: Option<ViewId>, rect: Rect, z: ZLayer, raw: bool,
        tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    ) -> (ViewId, WindowId) {
        let view = Self::prepare_view(s, doc, view, raw);
        let window =
            s.workspace.create_floating(rect, z, Box::new(DocViewRenderer::new(view, raw)));
        s.attach_view(window, view);
        ViewProtocol::resize(s);

        if let Some(tx) = tx {
            let _ = tx.send((view, window));
        }

        (view, window)
    }

    pub fn replace_window(
        s: &mut State, doc: DocId, view: Option<ViewId>, window: WindowId, raw: bool,
        tx: Option<oneshot::Sender<ViewId>>,
    ) -> ViewId {
        let view = Self::prepare_view(s, doc, view, raw);
        s.replace_view(window, view, Box::new(DocViewRenderer::new(view, raw)));
        ViewProtocol::resize(s);

        if let Some(tx) = tx {
            let _ = tx.send(view);
        }

        view
    }

    pub fn destroy_view(s: &mut State, view: ViewId) {
        if s.destroy_view(view).is_some() {
            ViewProtocol::resize(s);
        }
    }

    fn prepare_view(s: &mut State, doc: DocId, view: Option<ViewId>, raw: bool) -> ViewId {
        let view = view.unwrap_or_else(|| s.create_view(doc));

        if raw {
            let vse = s.view_store.get_mut(&view).unwrap();
            vse.layout.gutter = false;
            vse.layout.mode_line = 0;
        }

        view
    }
}
