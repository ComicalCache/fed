use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use fed_core::{CoreCommandSender, DocumentId};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    protocols::view::store::{LocalViewData, LocalViewStore},
    render::WindowId,
    state::{State, ViewId, ViewStoreTypes},
    types::Pos,
};

pub enum ViewCommand {
    Init { window: WindowId, view: ViewId, doc: DocumentId },
    ScrollTo { view: ViewId, pos: Pos },
    ScrollIfNeeded { view: ViewId, pos: Pos },
    Resize,
}

pub struct ViewProtocol {
    local_store: Arc<RwLock<LocalViewStore>>,
    state: State,

    rx: UnboundedReceiver<ViewCommand>,
    core_tx: CoreCommandSender,
}

impl ViewProtocol {
    pub fn new(
        state: State, rx: UnboundedReceiver<ViewCommand>, core_tx: CoreCommandSender,
    ) -> Self {
        Self { local_store: Arc::new(RwLock::new(HashMap::new())), state, rx, core_tx }
    }

    pub fn store(&self) -> Arc<RwLock<LocalViewStore>> { self.local_store.clone() }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ViewCommand::Init { window, view, doc } => self.init(window, view, doc).await,
                ViewCommand::ScrollTo { view, pos } => self.scroll_to(view, pos).await,
                ViewCommand::ScrollIfNeeded { view, pos } => self.scroll_if_needed(view, pos).await,
                ViewCommand::Resize => self.resize().await,
            }
        }
    }

    async fn init(&mut self, window: WindowId, view: ViewId, doc: DocumentId) {
        let (height, layout) = {
            let workspace = self.state.workspace.read().unwrap();
            let view_store = self.state.view_store.read().unwrap();

            let Some(height) = workspace.get_rect(window).map(|rect| rect.height) else {
                return;
            };
            let layout = view_store
                .get(&view)
                .and_then(|view| view.get::<ViewStoreTypes::Layout>())
                .copied()
                .unwrap_or_default();

            (height, layout)
        };

        let buffer_height = height.saturating_sub(layout.mode_line);
        if buffer_height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(Pos::default()), buffer_height).await;
        }
    }

    async fn scroll_to(&mut self, view: ViewId, pos: Pos) {
        let window = {
            let window_view_map = self.state.window_view_map.read().unwrap();
            window_view_map.iter().find(|&(_, &v)| v == view).map(|(&w, _)| w)
        };
        let Some(window) = window else {
            return;
        };

        let Some(rect) = self.state.workspace.read().unwrap().get_rect(window) else {
            return;
        };

        let (doc, layout) = {
            let view_store = self.state.view_store.read().unwrap();
            let Some(view) = view_store.get(&view) else {
                return;
            };

            let doc = view.get::<DocumentId>().cloned();
            let layout = view.get::<ViewStoreTypes::Layout>().copied().unwrap_or_default();

            (doc, layout)
        };
        let Some(doc) = doc else {
            return;
        };

        let buffer_height = rect.height.saturating_sub(layout.mode_line);
        if buffer_height == 0 {
            return;
        }

        let scroll = ViewStoreTypes::Scroll(pos);

        let mut view_store = self.state.view_store.write().unwrap();
        if let Some(view) = view_store.get_mut(&view) {
            view.insert(scroll);
        }
        drop(view_store);

        self.fetch(view, doc, scroll, buffer_height).await;
    }

    async fn scroll_if_needed(&mut self, view: ViewId, pos: Pos) {
        let window = {
            let window_view_map = self.state.window_view_map.read().unwrap();
            window_view_map.iter().find(|&(_, &v)| v == view).map(|(&w, _)| w)
        };
        let Some(window) = window else {
            return;
        };

        let Some(rect) = self.state.workspace.read().unwrap().get_rect(window) else {
            return;
        };

        let (doc, mut scroll, layout) = {
            let view_store = self.state.view_store.read().unwrap();
            let Some(view) = view_store.get(&view) else {
                return;
            };

            let doc = view.get::<DocumentId>().cloned();
            let scroll = view.get::<ViewStoreTypes::Scroll>().copied().unwrap_or_default();
            let layout = view.get::<ViewStoreTypes::Layout>().copied().unwrap_or_default();

            (doc, scroll, layout)
        };
        let Some(doc) = doc else {
            return;
        };

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
            let mut view_store = self.state.view_store.write().unwrap();
            if let Some(view) = view_store.get_mut(&view) {
                view.insert(scroll);
            }
            drop(view_store);

            self.fetch(view, doc, scroll, buffer_height).await;
        }
    }

    async fn resize(&mut self) {
        let entries: Vec<_> = {
            let workspace = self.state.workspace.read().unwrap();
            let view_store = self.state.view_store.read().unwrap();
            let window_view_map = self.state.window_view_map.read().unwrap();

            window_view_map
                .iter()
                .filter_map(|(&window, &view)| {
                    let Some(view_data) = view_store.get(&view) else {
                        return None;
                    };

                    let doc = view_data.get::<DocumentId>().copied()?;
                    let scroll =
                        view_data.get::<ViewStoreTypes::Scroll>().copied().unwrap_or_default();
                    let layout =
                        view_data.get::<ViewStoreTypes::Layout>().copied().unwrap_or_default();
                    let buffer_height =
                        workspace.get_rect(window)?.height.saturating_sub(layout.mode_line);

                    Some((view, doc, scroll, buffer_height))
                })
                .collect()
        };

        for (view, doc, scroll, height) in entries {
            if height > 0 {
                self.fetch(view, doc, scroll, height).await;
            }
        }
    }

    async fn fetch(
        &self, view: ViewId, doc: DocumentId, scroll: ViewStoreTypes::Scroll, height: usize,
    ) {
        if let Ok(lines_count) = self.core_tx.lines(doc).await {
            let digits = lines_count.checked_ilog10().unwrap_or(0) as usize + 1;
            let gutter = digits + 2;

            let mut view_store = self.state.view_store.write().unwrap();
            if let Some(view) = view_store.get_mut(&view) {
                let mut layout = view.get::<ViewStoreTypes::Layout>().copied().unwrap_or_default();
                if layout.gutter != gutter {
                    layout.gutter = gutter;
                    view.insert(layout);
                }
            }
        }

        let Ok(start) = self.core_tx.get_line_start_byte(doc, scroll.y).await else {
            return;
        };
        let Ok(end) = self.core_tx.get_line_end_byte(doc, scroll.y + height).await else {
            return;
        };

        let Ok(data) = self.core_tx.get_slice(doc, start..end).await else {
            return;
        };
        let mut lines = data.split_inclusive('\n').map(String::from).collect::<Vec<_>>();
        if data.ends_with('\n') {
            lines.push(String::new());
        }

        self.local_store.write().unwrap().insert(view, LocalViewData { offset: start, lines });
    }
}
