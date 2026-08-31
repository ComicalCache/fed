use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use fed_core::{CoreCommandSender, DocumentId};

use crate::{
    protocols::buffer::store::BufferStore,
    render::WindowId,
    state::{State, ViewId, ViewStoreTypes},
    types::Pos,
};

pub enum BufferCommand {
    Init { window: WindowId, view: ViewId, doc: DocumentId },
    ScrollTo { view: ViewId, pos: Pos },
    ScrollIfNeeded { view: ViewId, pos: Pos },
    Resize,
}

pub struct BufferProtocol {
    store: Arc<RwLock<BufferStore>>,
    state: State,

    rx: flume::Receiver<BufferCommand>,
    core_tx: CoreCommandSender,
}

impl BufferProtocol {
    pub fn new(
        state: State, rx: flume::Receiver<BufferCommand>, core_tx: CoreCommandSender,
    ) -> Self {
        Self { store: Arc::new(RwLock::new(HashMap::new())), state, rx, core_tx }
    }

    pub fn store(&self) -> Arc<RwLock<BufferStore>> { self.store.clone() }

    pub async fn run(&mut self) {
        while let Ok(cmd) = self.rx.recv_async().await {
            match cmd {
                BufferCommand::Init { window, view, doc } => self.init(window, view, doc).await,
                BufferCommand::ScrollTo { view, pos } => self.scroll_to(view, pos).await,
                BufferCommand::ScrollIfNeeded { view, pos } => {
                    self.scroll_if_needed(view, pos).await
                }
                BufferCommand::Resize => self.resize().await,
            }
        }
    }

    async fn init(&mut self, window: WindowId, view: ViewId, doc: DocumentId) {
        let height = {
            let workspace = self.state.workspace.read().unwrap();
            let Some(height) = workspace.get_rect(window).map(|rect| rect.height) else {
                return;
            };

            height
        };

        if height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(Pos::default()), height).await;
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
        if rect.height == 0 {
            return;
        }

        let doc = {
            let view_store = self.state.view_store.read().unwrap();
            let Some(view) = view_store.get(&view) else {
                return;
            };

            view.get::<DocumentId>().cloned()
        };
        let Some(doc) = doc else {
            return;
        };

        let scroll = ViewStoreTypes::Scroll(pos);

        let mut view_store = self.state.view_store.write().unwrap();
        if let Some(view) = view_store.get_mut(&view) {
            view.insert(scroll);
        }
        drop(view_store);

        self.fetch(view, doc, scroll, rect.height).await;
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
        if rect.height == 0 {
            return;
        }

        let doc = {
            let view_store = self.state.view_store.read().unwrap();
            let Some(view) = view_store.get(&view) else {
                return;
            };

            view.get::<DocumentId>().cloned()
        };
        let Some(doc) = doc else {
            return;
        };

        let mut scroll = {
            let view_store = self.state.view_store.read().unwrap();
            view_store
                .get(&view)
                .and_then(|view| view.get::<ViewStoreTypes::Scroll>())
                .map(|&scroll| scroll)
                .unwrap_or_default()
        };

        let mut needed = false;

        if pos.y < scroll.y {
            scroll.y = pos.y;
            needed = true;
        } else if pos.y >= scroll.y + rect.height {
            scroll.y = pos.y.saturating_sub(rect.height).saturating_add(1);
            needed = true;
        }

        if pos.x < scroll.x {
            scroll.x = pos.x;
            needed = true;
        } else if pos.x >= scroll.x + rect.width {
            scroll.x = pos.x.saturating_sub(rect.width).saturating_add(1);
            needed = true;
        }

        if needed {
            let mut view_store = self.state.view_store.write().unwrap();
            if let Some(v) = view_store.get_mut(&view) {
                v.insert(scroll);
            }
            drop(view_store);

            self.fetch(view, doc, scroll, rect.height).await;
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
                    let scroll = view_store
                        .get(&view)
                        .and_then(|view| view.get::<ViewStoreTypes::Scroll>())
                        .map(|&scroll| scroll)
                        .unwrap_or_default();

                    let doc = {
                        let Some(view) = view_store.get(&view) else {
                            return None;
                        };

                        view.get::<DocumentId>()
                    };
                    let Some(&doc) = doc else {
                        return None;
                    };

                    workspace.get_rect(window).map(|rect| (view, doc, scroll, rect.height))
                })
                .collect()
        };

        for (view, doc, scroll, height) in entries {
            self.fetch(view, doc, scroll, height).await;
        }
    }

    async fn fetch(
        &self, view: ViewId, doc: DocumentId, scroll: ViewStoreTypes::Scroll, height: usize,
    ) {
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

        self.store.write().unwrap().insert(view, lines);
    }
}
