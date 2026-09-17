use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use piece_table::Slice;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    protocols::{
        screen::ScreenCommand,
        view::{
            ViewRenderer,
            store::{LocalViewData, LocalViewStore},
        },
        view_decorator::ViewDecoratorRenderer,
    },
    render::{WindowId, ZLayer},
    state::{DocumentId, StateLock, ViewId, ViewStoreTypes},
    types::{Pos, Rect, RectSplit},
};

pub enum ViewCommand {
    Init {
        window: WindowId,
        view: ViewId,
        doc: DocumentId,
    },
    Update {
        view: ViewId,
    },
    ScrollTo {
        view: ViewId,
        pos: Pos,
    },
    ScrollIfNeeded {
        view: ViewId,
        pos: Pos,
    },

    CreateRawTile {
        doc: DocumentId,
        view: Option<ViewId>,
        split: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateTile {
        doc: DocumentId,
        view: Option<ViewId>,
        split: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateFloating {
        doc: DocumentId,
        view: Option<ViewId>,
        rect: Rect,
        z: ZLayer,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateRawFloating {
        doc: DocumentId,
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

pub struct ViewProtocol {
    local_store: Arc<RwLock<LocalViewStore>>,

    state_lock: StateLock,

    rx: UnboundedReceiver<ViewCommand>,
    screen_tx: UnboundedSender<ScreenCommand>,
}

impl ViewProtocol {
    pub fn new(
        state_lock: StateLock, rx: UnboundedReceiver<ViewCommand>,
        screen_tx: UnboundedSender<ScreenCommand>,
    ) -> Self {
        Self { local_store: Arc::new(RwLock::new(HashMap::new())), state_lock, rx, screen_tx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ViewCommand::Init { window, view, doc } => self.init(window, view, doc).await,
                ViewCommand::Update { view } => self.update(view).await,
                ViewCommand::ScrollTo { view, pos } => self.scroll_to(view, pos).await,
                ViewCommand::ScrollIfNeeded { view, pos } => self.scroll_if_needed(view, pos).await,

                ViewCommand::CreateRawTile { doc, view, split, tx } => {
                    self.create_tile(doc, view, split, tx, true).await
                }
                ViewCommand::CreateTile { doc, view, split, tx } => {
                    self.create_tile(doc, view, split, tx, false).await
                }
                ViewCommand::CreateRawFloating { doc, view, rect, z, tx } => {
                    self.create_floating(doc, view, rect, z, tx, true).await
                }
                ViewCommand::CreateFloating { doc, view, rect, z, tx } => {
                    self.create_floating(doc, view, rect, z, tx, false).await
                }
                ViewCommand::DestroyView { view } => self.destroy_view(view).await,

                ViewCommand::Resize => self.resize().await,
            }

            // Always redraw the screen after any view command.
            let _ = self.screen_tx.send(ScreenCommand::Render);
        }
    }

    async fn init(&mut self, window: WindowId, view: ViewId, doc: DocumentId) {
        let state = self.state_lock.read();

        let Some(height) = state.workspace.get_rect(window).map(|r| r.height) else { return };
        let Some(vse) = state.view_store.get(&view) else { return };
        let layout = vse.layout;

        drop(state);

        let buffer_height = height.saturating_sub(layout.mode_line);
        if buffer_height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(Pos::default()), buffer_height).await;
        }
    }

    async fn update(&mut self, view: ViewId) {
        let state = self.state_lock.read();

        let Some(window) = state.window_view_map.iter().find(|&(_, &v)| v == view).map(|(&w, _)| w)
        else {
            return;
        };
        let Some(rect) = state.workspace.get_rect(window) else { return };
        let Some(vse) = state.view_store.get(&view) else { return };
        let doc = vse.doc;
        let scroll = vse.scroll;
        let layout = vse.layout;

        drop(state);

        let buffer_height = rect.height.saturating_sub(layout.mode_line);
        if buffer_height > 0 {
            self.fetch(view, doc, scroll, buffer_height).await;
        }
    }

    async fn scroll_to(&mut self, view: ViewId, pos: Pos) {
        let mut state = self.state_lock.write();

        let Some(window) = state.window_view_map.iter().find(|&(_, &v)| v == view).map(|(&w, _)| w)
        else {
            return;
        };
        let Some(rect) = state.workspace.get_rect(window) else { return };
        let Some(vse) = state.view_store.get_mut(&view) else { return };
        let doc = vse.doc;
        let layout = vse.layout;

        let buffer_height = rect.height.saturating_sub(layout.mode_line);
        if buffer_height == 0 {
            return;
        }

        vse.scroll = ViewStoreTypes::Scroll(pos);

        drop(state);

        self.fetch(view, doc, ViewStoreTypes::Scroll(pos), buffer_height).await;
    }

    async fn scroll_if_needed(&mut self, view: ViewId, pos: Pos) {
        let state = self.state_lock.read();

        let Some(window) = state.window_view_map.iter().find(|&(_, &v)| v == view).map(|(&w, _)| w)
        else {
            return;
        };
        let Some(rect) = state.workspace.get_rect(window) else { return };
        let Some((vse, dse)) = state.view_and_doc(view) else { return };

        let doc = vse.doc;
        let lines = dse.doc.data.lines();
        let mut scroll = vse.scroll;
        let layout = vse.layout;

        drop(state);

        let buffer_width = rect.width.saturating_sub(layout.gutter_width(lines));
        let buffer_height = rect.height.saturating_sub(layout.mode_line);
        if buffer_width == 0 || buffer_height == 0 {
            return;
        }

        let mut needed = false;

        if pos.y < scroll.y {
            scroll.y = pos.y;
            needed = true;
        } else if pos.y >= scroll.y + buffer_height {
            scroll.y = pos.y.saturating_sub(buffer_height).saturating_add(1);
            needed = true;
        }

        if pos.x < scroll.x {
            scroll.x = pos.x;
            needed = true;
        } else if pos.x >= scroll.x + buffer_width {
            scroll.x = pos.x.saturating_sub(buffer_width).saturating_add(1);
            needed = true;
        }

        if needed {
            let mut state = self.state_lock.write();

            let Some(vse) = state.view_store.get_mut(&view) else { return };
            vse.scroll = scroll;

            drop(state);

            self.fetch(view, doc, scroll, buffer_height).await;
        }
    }

    async fn create_tile(
        &mut self, doc: DocumentId, view: Option<ViewId>, split: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>, raw: bool,
    ) {
        let mut state = self.state_lock.write();

        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window = if raw {
            let renderer = Box::new(ViewRenderer::new(
                doc,
                view,
                self.local_store.clone(),
                self.state_lock.clone(),
            ));

            state.workspace.create_tile(split, renderer)
        } else {
            let renderer = Box::new(ViewDecoratorRenderer::new(
                view,
                ViewRenderer::new(doc, view, self.local_store.clone(), self.state_lock.clone()),
                self.state_lock.clone(),
            ));

            state.workspace.create_tile(split, renderer)
        };
        state.window_view_map.insert(window, view);

        drop(state);

        self.init(window, view, doc).await;

        let _ = tx.send((view, window));
    }

    async fn create_floating(
        &mut self, doc: DocumentId, view: Option<ViewId>, rect: Rect, z: ZLayer,
        tx: oneshot::Sender<(ViewId, WindowId)>, raw: bool,
    ) {
        let mut state = self.state_lock.write();

        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window = if raw {
            let renderer = Box::new(ViewRenderer::new(
                doc,
                view,
                self.local_store.clone(),
                self.state_lock.clone(),
            ));

            state.workspace.create_floating(rect, z, renderer)
        } else {
            let renderer = Box::new(ViewDecoratorRenderer::new(
                view,
                ViewRenderer::new(doc, view, self.local_store.clone(), self.state_lock.clone()),
                self.state_lock.clone(),
            ));

            state.workspace.create_floating(rect, z, renderer)
        };
        state.window_view_map.insert(window, view);

        drop(state);

        self.init(window, view, doc).await;

        let _ = tx.send((view, window));
    }

    async fn destroy_view(&mut self, view: ViewId) {
        let mut state = self.state_lock.write();
        let mut local_store = self.local_store.write().unwrap();

        let windows: Vec<_> =
            state.window_view_map.iter().filter(|&(_, &v)| v == view).map(|(&k, _)| k).collect();
        for window in windows {
            state.workspace.destroy_window(window);
            state.window_view_map.remove(&window);

            local_store.remove(&view);
        }

        drop(local_store);
        drop(state);
    }

    async fn resize(&mut self) {
        let state = self.state_lock.read();

        let mappings: Vec<_> = state.window_view_map.iter().map(|(&w, &v)| (w, v)).collect();

        let mut entries = Vec::new();
        for (window, view) in mappings {
            let Some(rect) = state.workspace.get_rect(window) else { continue };
            let Some(vse) = state.view_store.get(&view) else { continue };
            let doc = vse.doc;
            let scroll = vse.scroll;
            let layout = vse.layout;

            let height = rect.height.saturating_sub(layout.mode_line);

            entries.push((view, doc, scroll, height));
        }

        drop(state);

        for (view, doc, scroll, height) in entries {
            if height > 0 {
                self.fetch(view, doc, scroll, height).await;
            }
        }
    }

    async fn fetch(
        &self, view: ViewId, doc: DocumentId, scroll: ViewStoreTypes::Scroll, height: usize,
    ) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(vse) = state.view_store.get_mut(&view) else { return };
        let Some(dse) = state.document_store.get_mut(&doc) else { return };

        let start = dse.doc.data.get_line_start_byte(scroll.y);
        let end = dse.doc.data.get_line_end_byte(scroll.y + height);

        vse.decs.update(start, end);
        dse.decs.update(start, end);

        let data = dse.doc.data.slice(start..end);

        drop(guard);

        let mut lines = data.split_inclusive('\n').map(String::from).collect::<Vec<_>>();
        if data.is_empty() || data.ends_with('\n') {
            lines.push(String::new());
        }

        self.local_store.write().unwrap().insert(view, LocalViewData { offset: start, lines });
    }
}
