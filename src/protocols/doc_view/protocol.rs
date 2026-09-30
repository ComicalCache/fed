use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    debug_panic::debug_panic,
    protocols::{
        doc_view::{DocViewCommand, DocViewRenderer},
        view::ViewCommand,
    },
    render::{WindowId, ZLayer},
    state::{DocId, StateLock, ViewId},
    types::{Rect, RectSplit},
};

pub struct DocViewProtocol {
    state_lock: StateLock,

    rx: UnboundedReceiver<DocViewCommand>,
    view_tx: UnboundedSender<ViewCommand>,
}

impl DocViewProtocol {
    pub fn new(
        state_lock: StateLock, rx: UnboundedReceiver<DocViewCommand>,
        view_tx: UnboundedSender<ViewCommand>,
    ) -> Self {
        Self { state_lock, rx, view_tx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                DocViewCommand::CreateTile { doc, view, split_window, direction, raw, tx } => {
                    self.create_tile(doc, view, split_window, direction, tx, raw)
                }
                DocViewCommand::CreateFloating { doc, view, rect, z, raw, tx } => {
                    self.create_floating(doc, view, rect, z, tx, raw)
                }
                DocViewCommand::DestroyView { view } => self.destroy_view(view),
            }
        }
    }

    fn create_tile(
        &self, doc: DocId, view: Option<ViewId>, split_window: WindowId, direction: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>, raw: bool,
    ) {
        let mut state = self.state_lock.write();
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

            return;
        };

        state.index.link_window_to_view(window, view);
        drop(state);

        let _ = self.view_tx.send(ViewCommand::Init { window, view, doc });
        let _ = tx.send((view, window));
    }

    fn create_floating(
        &self, doc: DocId, view: Option<ViewId>, rect: Rect, z: ZLayer,
        tx: oneshot::Sender<(ViewId, WindowId)>, raw: bool,
    ) {
        let mut state = self.state_lock.write();
        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window =
            state.workspace.create_floating(rect, z, Box::new(DocViewRenderer::new(view, raw)));

        state.index.link_window_to_view(window, view);
        drop(state);

        let _ = self.view_tx.send(ViewCommand::Init { window, view, doc });
        let _ = tx.send((view, window));
    }

    fn destroy_view(&self, view: ViewId) {
        let mut state = self.state_lock.write();

        for window in state.index.unlink_view(view) {
            state.workspace.destroy_window(window);
        }

        drop(state);
    }
}
