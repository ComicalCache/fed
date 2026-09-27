use std::sync::atomic::{AtomicUsize, Ordering};

use piece_table::Slice;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    debug_panic::debug_panic,
    protocols::{
        action::ActionCommand,
        mini_buffer::{MiniBufferCommand, decorations::MiniBufferDecorationProvider},
        view::ViewCommand,
    },
    render::ZLayer,
    state::{MiniBufferId, MiniBufferStoreTypes, StateLock, ViewStoreTypes},
    types::{Pos, Rect},
};

pub struct MiniBufferProtocol {
    next_id: AtomicUsize,

    width: usize,
    height: usize,

    state_lock: StateLock,

    rx: UnboundedReceiver<MiniBufferCommand>,
    action_tx: UnboundedSender<ActionCommand>,
    view_tx: UnboundedSender<ViewCommand>,
}

impl MiniBufferProtocol {
    pub fn new(
        width: usize, height: usize, state_lock: StateLock,
        rx: UnboundedReceiver<MiniBufferCommand>, action_tx: UnboundedSender<ActionCommand>,
        view_tx: UnboundedSender<ViewCommand>,
    ) -> Self {
        Self { next_id: AtomicUsize::new(1), width, height, state_lock, rx, action_tx, view_tx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                MiniBufferCommand::Message { message, tx } => self.message(message, tx).await,
                MiniBufferCommand::Prompt { prompt, initial_text, id_tx, res_tx } => {
                    self.prompt(prompt, initial_text, id_tx, res_tx).await
                }
                MiniBufferCommand::Submit => self.submit(),
                MiniBufferCommand::Close { id } => self.close(id),
                MiniBufferCommand::Resize { width, height } => self.resize(width, height),
            }
        }
    }

    async fn message(&self, message: String, tx: oneshot::Sender<MiniBufferId>) {
        let state = self.state_lock.read();
        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let id = state.mini_buffer_store.id;
        let doc = state.mini_buffer_store.doc;
        let view = state.mini_buffer_store.view;
        drop(state);

        self.close(id);

        let (floating_tx, floating_rx) = oneshot::channel();
        let _ = self.view_tx.send(ViewCommand::CreateRawFloating {
            doc,
            view: Some(view),
            rect: Rect::new(Pos::new(0, self.height.saturating_sub(1)), self.width, 1),
            z: ZLayer::MiniBuffer,
            tx: floating_tx,
        });
        let Ok((_, window)) = floating_rx.await else { return };

        let _ = self.action_tx.send(ActionCommand::InsertAt { view, text: message, offset: 0 });
        let _ = self.action_tx.send(ActionCommand::Saved { doc });

        let id = MiniBufferId(self.next_id.fetch_add(1, Ordering::Relaxed));

        let mut state = self.state_lock.write();
        state.mini_buffer_store.id = id;
        state.mini_buffer_store.kind = MiniBufferStoreTypes::Kind::Message;
        state.mini_buffer_store.window = Some(window);
        state.mini_buffer_store.prev_window = state.workspace.active_window;
        drop(state);

        let _ = tx.send(id);
    }

    async fn prompt(
        &self, prompt: String, initial_text: Option<String>, id_tx: oneshot::Sender<MiniBufferId>,
        res_tx: oneshot::Sender<String>,
    ) {
        let state = self.state_lock.read();
        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let id = state.mini_buffer_store.id;
        let doc = state.mini_buffer_store.doc;
        let view = state.mini_buffer_store.view;
        drop(state);

        self.close(id);

        let _ = self.action_tx.send(ActionCommand::CreateCursorAtPos { view, pos: Pos::new(0, 0) });

        if let Some(text) = initial_text {
            let _ = self.action_tx.send(ActionCommand::InsertAt { view, text, offset: 0 });
        }

        let (floating_tx, floating_rx) = oneshot::channel();
        let _ = self.view_tx.send(ViewCommand::CreateRawFloating {
            doc,
            view: Some(view),
            rect: Rect::new(Pos::new(0, self.height.saturating_sub(1)), self.width, 1),
            z: ZLayer::MiniBuffer,
            tx: floating_tx,
        });
        let Ok((_, window)) = floating_rx.await else { return };

        let id = MiniBufferId(self.next_id.fetch_add(1, Ordering::Relaxed));

        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(vse) = state.view_store.get_mut(&view) else {
            state.workspace.destroy_window(window);

            debug_panic!();

            return;
        };

        vse.decs.layers.insert(
            ViewStoreTypes::DecorationId::MiniBuffer,
            Box::new(MiniBufferDecorationProvider::new(prompt, state.theme.mini_buffer)),
        );

        state.mini_buffer_store.id = id;
        state.mini_buffer_store.kind = MiniBufferStoreTypes::Kind::Prompt;
        state.mini_buffer_store.window = Some(window);
        state.mini_buffer_store.prev_window = state.workspace.active_window;
        state.mini_buffer_store.res_tx = Some(res_tx);

        state.workspace.active_window = Some(window);
        drop(guard);

        let _ = id_tx.send(id);
    }

    fn submit(&self) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        if state.mini_buffer_store.kind != MiniBufferStoreTypes::Kind::Prompt {
            return;
        }

        let doc = state.mini_buffer_store.doc;
        let id = state.mini_buffer_store.id;

        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };
        let Some(res_tx) = state.mini_buffer_store.res_tx.take() else {
            debug_panic!();
            return;
        };

        let response = dse.doc.data.slice(0..dse.doc.data.len());
        drop(guard);

        let _ = res_tx.send(response);

        self.close(id);
    }

    fn close(&self, id: MiniBufferId) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::None {
            return;
        }
        if state.mini_buffer_store.id != id {
            return;
        }

        if let Some(window) = state.mini_buffer_store.window.take() {
            state.index.unlink_window(window);
            state.workspace.destroy_window(window);
        };

        state.workspace.active_window = state.mini_buffer_store.prev_window;

        state.mini_buffer_store.kind = MiniBufferStoreTypes::Kind::None;
        state.mini_buffer_store.prev_window = None;
        state.mini_buffer_store.res_tx = None;

        let view = state.mini_buffer_store.view;
        let doc = state.mini_buffer_store.doc;
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        vse.decs.layers.remove(&ViewStoreTypes::DecorationId::MiniBuffer);

        let len = dse.doc.data.len();
        let cursors: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();
        drop(guard);

        for offset in cursors {
            let _ =
                self.action_tx.send(ActionCommand::RemoveCursors { view, offsets: vec![offset] });
        }
        let _ = self.action_tx.send(ActionCommand::Remove { view, offset: 0, len });
        let _ = self.action_tx.send(ActionCommand::Saved { doc });
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        let mut state = self.state_lock.write();
        if state.mini_buffer_store.kind == MiniBufferStoreTypes::Kind::None {
            return;
        }

        let Some(window) = state.mini_buffer_store.window else {
            debug_panic!();
            return;
        };

        state.workspace.resize_floating(window, width, 1);
        drop(state);
    }
}
