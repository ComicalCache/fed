use tokio::sync::oneshot;

use crate::{
    debug_panic::debug_panic,
    protocols::{
        doc_view::{DocViewCmd, DocViewRenderer},
        view::ViewProtocol,
    },
    render::{WindowId, ZLayer},
    state::{DocId, State, ViewId},
    types::{Rect, RectSplit},
};

pub struct DocViewProtocol {}

impl DocViewProtocol {
    pub fn exec(state: &mut State, cmd: DocViewCmd) {
        match cmd {
            DocViewCmd::CreateTile { doc, view, split_window, direction, raw, tx } => {
                Self::create_tile(state, doc, view, split_window, direction, raw, tx);
            }
            DocViewCmd::CreateFloating { doc, view, rect, z, raw, tx } => {
                Self::create_floating(state, doc, view, rect, z, raw, tx);
            }
            DocViewCmd::ReplaceWindow { doc, view, window, raw, tx } => {
                Self::replace_window(state, doc, view, window, raw, tx);
            }
            DocViewCmd::DestroyView { view } => Self::destroy_view(state, view),
        }
    }

    pub fn create_tile(
        state: &mut State, doc: DocId, view: Option<ViewId>, split_window: WindowId,
        direction: RectSplit, raw: bool, tx: Option<oneshot::Sender<Option<(ViewId, WindowId)>>>,
    ) -> Option<(ViewId, WindowId)> {
        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window = state.workspace.create_tile(
            split_window,
            direction,
            Box::new(DocViewRenderer::new(view, raw)),
        );

        let Some(window) = window else {
            let windows = state.destroy_view(view);
            debug_assert!(windows.is_empty());

            debug_panic!();

            if let Some(tx) = tx {
                let _ = tx.send(None);
            }

            return None;
        };

        state.index.link_window_to_view(window, view);

        ViewProtocol::update(state, view);

        if let Some(tx) = tx {
            let _ = tx.send(Some((view, window)));
        }

        Some((view, window))
    }

    pub fn create_floating(
        state: &mut State, doc: DocId, view: Option<ViewId>, rect: Rect, z: ZLayer, raw: bool,
        tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    ) -> (ViewId, WindowId) {
        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window =
            state.workspace.create_floating(rect, z, Box::new(DocViewRenderer::new(view, raw)));
        state.index.link_window_to_view(window, view);

        ViewProtocol::update(state, view);

        if let Some(tx) = tx {
            let _ = tx.send((view, window));
        }

        (view, window)
    }

    pub fn replace_window(
        state: &mut State, doc: DocId, view: Option<ViewId>, window: WindowId, raw: bool,
        tx: Option<oneshot::Sender<ViewId>>,
    ) -> ViewId {
        let view = view.unwrap_or_else(|| state.create_view(doc));

        state.workspace.replace_renderer(window, Box::new(DocViewRenderer::new(view, raw)));
        state.index.link_window_to_view(window, view);

        ViewProtocol::update(state, view);

        if let Some(tx) = tx {
            let _ = tx.send(view);
        }

        view
    }

    pub fn destroy_view(state: &mut State, view: ViewId) {
        for window in state.index.unlink_view(view) {
            state.workspace.destroy_window(window);
        }
    }
}
