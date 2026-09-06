use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use piece_table::Slice;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    protocols::{
        screen::ScreenCommand,
        view::{
            ViewRenderer,
            store::{LocalViewData, LocalViewStore},
        },
        view_decorator::ViewDecoratorRenderer,
    },
    render::WindowId,
    state::{DocumentId, StateLock, ViewId, ViewStoreTypes},
    types::{Pos, RectSplit},
};

pub enum ViewCommand {
    Init { window: WindowId, view: ViewId, doc: DocumentId },
    Update { view: ViewId },
    ScrollTo { view: ViewId, pos: Pos },
    ScrollIfNeeded { view: ViewId, pos: Pos },
    SpawnWindow { doc: DocumentId, split: RectSplit },
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
                ViewCommand::SpawnWindow { doc, split } => self.spawn_window(doc, split).await,
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
        let Some(vse) = state.view_store.get(&view) else { return };
        let doc = vse.doc;
        let mut scroll = vse.scroll;
        let layout = vse.layout;

        drop(state);

        let buffer_width = rect.width.saturating_sub(layout.gutter);
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

    async fn spawn_window(&mut self, doc: DocumentId, split: RectSplit) {
        let mut state = self.state_lock.write();

        let view = state.create_view(doc);

        let view_renderer =
            ViewRenderer::new(doc, view, self.local_store.clone(), self.state_lock.clone());
        let view_decorator_renderer =
            Box::new(ViewDecoratorRenderer::new(view, view_renderer, self.state_lock.clone()));

        let window = state.workspace.create_tile(view_decorator_renderer, split);
        state.window_view_map.insert(window, view);

        drop(state);

        self.init(window, view, doc).await;
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

            let buffer_height = rect.height.saturating_sub(layout.mode_line);

            entries.push((view, doc, scroll, buffer_height));
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

        let Some(dse) = state.document_store.get(&doc) else { return };
        let lines = dse.doc.data.lines();

        let digits = lines.checked_ilog10().unwrap_or(0) as usize + 1;
        let gutter = digits + 2;

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        if vse.layout.gutter != gutter {
            vse.layout.gutter = gutter;
        }

        let start = dse.doc.data.get_line_start_byte(scroll.y);
        let end = dse.doc.data.get_line_end_byte(scroll.y + height);
        let data = dse.doc.data.slice(start..end);

        drop(guard);

        let mut lines = data.split_inclusive('\n').map(String::from).collect::<Vec<_>>();
        if data.ends_with('\n') {
            lines.push(String::new());
        }

        self.local_store.write().unwrap().insert(view, LocalViewData { offset: start, lines });
    }
}
