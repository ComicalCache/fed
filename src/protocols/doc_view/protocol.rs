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
    util::debug_panic,
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
        direction: RectSplit, raw: bool, tx: Option<oneshot::Sender<Option<(ViewId, WindowId)>>>,
    ) -> Option<(ViewId, WindowId)> {
        let view = view.unwrap_or_else(|| s.create_view(doc));

        let window =
            s.workspace.create_tile(split_window, direction, Box::new(DocViewRenderer::new(raw)));

        let Some(window) = window else {
            Self::destroy_view(s, view);

            debug_panic!();

            if let Some(tx) = tx {
                let _ = tx.send(None);
            }

            return None;
        };

        s.index.link_window_to_view(window, view);

        ViewProtocol::update(s, view);

        if let Some(tx) = tx {
            let _ = tx.send(Some((view, window)));
        }

        Some((view, window))
    }

    pub fn create_floating(
        s: &mut State, doc: DocId, view: Option<ViewId>, rect: Rect, z: ZLayer, raw: bool,
        tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    ) -> (ViewId, WindowId) {
        let view = view.unwrap_or_else(|| s.create_view(doc));

        let window = s.workspace.create_floating(rect, z, Box::new(DocViewRenderer::new(raw)));
        s.index.link_window_to_view(window, view);

        ViewProtocol::update(s, view);

        if let Some(tx) = tx {
            let _ = tx.send((view, window));
        }

        (view, window)
    }

    pub fn replace_window(
        s: &mut State, doc: DocId, view: Option<ViewId>, window: WindowId, raw: bool,
        tx: Option<oneshot::Sender<ViewId>>,
    ) -> ViewId {
        let view = view.unwrap_or_else(|| s.create_view(doc));

        s.workspace.replace_renderer(window, Box::new(DocViewRenderer::new(raw)));
        s.index.link_window_to_view(window, view);

        ViewProtocol::update(s, view);

        if let Some(tx) = tx {
            let _ = tx.send(view);
        }

        view
    }

    pub fn destroy_view(s: &mut State, view: ViewId) {
        for window in s.destroy_view(view) {
            s.workspace.destroy_window(window);
        }
    }
}
