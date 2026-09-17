use std::sync::atomic::{AtomicUsize, Ordering};

use piece_table::Slice;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    protocols::{
        action::ActionCommand, mini_buffer::MiniBufferDecorationProvider, screen::ScreenCommand,
        view::ViewCommand,
    },
    render::ZLayer,
    state::{MiniBufferId, MiniBufferStoreTypes, StateLock, ViewStoreTypes::ViewDecoration},
    types::{Face, Pos, Rect},
};

pub enum MiniBufferCommand {
    Message { message: String, tx: oneshot::Sender<MiniBufferId> },
    Prompt { prompt: String, id_tx: oneshot::Sender<MiniBufferId>, res_tx: oneshot::Sender<String> },
    Submit,
    Close { id: MiniBufferId },
    Resize { width: usize, height: usize },
}

pub struct MiniBufferProtocol {
    next_id: AtomicUsize,
    width: usize,
    height: usize,

    state_lock: StateLock,

    rx: UnboundedReceiver<MiniBufferCommand>,
    action_tx: UnboundedSender<ActionCommand>,
    view_tx: UnboundedSender<ViewCommand>,
    screen_tx: UnboundedSender<ScreenCommand>,
}

impl MiniBufferProtocol {
    pub fn new(
        width: usize, height: usize, state_lock: StateLock,
        rx: UnboundedReceiver<MiniBufferCommand>, action_tx: UnboundedSender<ActionCommand>,
        view_tx: UnboundedSender<ViewCommand>, screen_tx: UnboundedSender<ScreenCommand>,
    ) -> Self {
        Self {
            next_id: AtomicUsize::new(1),
            width,
            height,
            state_lock,
            rx,
            action_tx,
            view_tx,
            screen_tx,
        }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                MiniBufferCommand::Message { message, tx } => self.message(message, tx).await,
                MiniBufferCommand::Prompt { prompt, id_tx, res_tx } => {
                    self.prompt(prompt, id_tx, res_tx).await
                }
                MiniBufferCommand::Submit => self.submit().await,
                MiniBufferCommand::Close { id } => self.close(id).await,
                MiniBufferCommand::Resize { width, height } => self.resize(width, height).await,
            }

            // Always redraw the screen after any mini buffer command.
            let _ = self.screen_tx.send(ScreenCommand::Render);
        }
    }

    async fn message(&mut self, message: String, tx: oneshot::Sender<MiniBufferId>) {
        let mut state = self.state_lock.write();

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let doc = state.mini_buffer_store.doc;
        let view = state.mini_buffer_store.view;

        state.mini_buffer_store.kind = MiniBufferStoreTypes::Kind::Message;
        state.mini_buffer_store.prev_window = state.workspace.active_window;

        drop(state);

        let _ = self.action_tx.send(ActionCommand::InsertAt { view, text: message, offset: 0 });

        let (floating_tx, floating_rx) = oneshot::channel();
        let _ = self.view_tx.send(ViewCommand::CreateRawFloating {
            doc,
            view: Some(view),
            rect: Rect::new(Pos::new(0, self.height.saturating_sub(1)), self.width, 1),
            z: ZLayer::MiniBuffer,
            tx: floating_tx,
        });
        let Ok((_, window)) = floating_rx.await else { return };

        let mut state = self.state_lock.write();

        let id = MiniBufferId(self.next_id.fetch_add(1, Ordering::Relaxed));

        state.mini_buffer_store.window = Some(window);
        state.mini_buffer_store.id = id;

        drop(state);

        let _ = tx.send(id);
    }

    async fn prompt(
        &mut self, prompt: String, id_tx: oneshot::Sender<MiniBufferId>,
        res_tx: oneshot::Sender<String>,
    ) {
        let mut state = self.state_lock.write();

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let doc = state.mini_buffer_store.doc;
        let view = state.mini_buffer_store.view;

        state.mini_buffer_store.kind = MiniBufferStoreTypes::Kind::Prompt;
        state.mini_buffer_store.prev_window = state.workspace.active_window;
        state.mini_buffer_store.res_tx = Some(res_tx);

        drop(state);

        let _ = self.action_tx.send(ActionCommand::CreateCursor { view, pos: Pos::new(0, 0) });

        let (floating_tx, floating_rx) = oneshot::channel();
        let _ = self.view_tx.send(ViewCommand::CreateRawFloating {
            doc,
            view: Some(view),
            rect: Rect::new(Pos::new(0, self.height.saturating_sub(1)), self.width, 1),
            z: ZLayer::MiniBuffer,
            tx: floating_tx,
        });
        let Ok((_, window)) = floating_rx.await else { return };

        let mut state = self.state_lock.write();

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.decs.layers.insert(
            ViewDecoration::MiniBuffer,
            Box::new(MiniBufferDecorationProvider::new(prompt, Face::default())),
        );

        let id = MiniBufferId(self.next_id.fetch_add(1, Ordering::Relaxed));

        state.mini_buffer_store.window = Some(window);
        state.mini_buffer_store.id = id;
        state.workspace.active_window = Some(window);

        drop(state);

        let _ = id_tx.send(id);
    }

    async fn submit(&mut self) {
        let mut state = self.state_lock.write();

        if state.mini_buffer_store.kind != MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let doc = state.mini_buffer_store.doc;
        let id = state.mini_buffer_store.id;

        let Some(dse) = state.document_store.get(&doc) else { return };

        let response = dse.doc.data.slice(0..dse.doc.data.len());

        let Some(res_tx) = state.mini_buffer_store.res_tx.take() else { return };

        drop(state);

        let _ = res_tx.send(response);

        self.close(id).await;
    }

    async fn close(&mut self, id: MiniBufferId) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::None {
            return;
        }
        if state.mini_buffer_store.id != id {
            return;
        }

        let Some(window) = state.mini_buffer_store.window.take() else { return };

        state.workspace.destroy_window(window);
        state.workspace.active_window = state.mini_buffer_store.prev_window;
        state.window_view_map.remove(&window);

        state.mini_buffer_store.kind = MiniBufferStoreTypes::Kind::None;
        state.mini_buffer_store.prev_window = None;
        state.mini_buffer_store.res_tx = None;

        let doc = state.mini_buffer_store.doc;
        let view = state.mini_buffer_store.view;

        let Some(vse) = state.view_store.get_mut(&view) else { return };
        let Some(dse) = state.document_store.get(&doc) else { return };

        vse.decs.layers.remove(&ViewDecoration::MiniBuffer);

        let len = dse.doc.data.len();
        let cursors: Vec<_> = vse.cursors.list.iter().map(|c| c.pos).collect();

        drop(guard);

        for pos in cursors {
            let _ = self.action_tx.send(ActionCommand::RemoveCursor { view, pos });
        }
        let _ = self.action_tx.send(ActionCommand::Remove { view, offset: 0, len });
    }

    async fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        let mut state = self.state_lock.write();

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::None {
            return;
        }

        let Some(window) = state.mini_buffer_store.window else { return };

        state.workspace.resize_floating(window, width, 1);

        drop(state);
    }
}
