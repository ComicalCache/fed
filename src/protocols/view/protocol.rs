use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use piece_table::Slice;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    protocols::{
        view::{
            ViewRenderer,
            store::{LocalViewData, LocalViewStore},
        },
        view_decorator::ViewDecoratorRenderer,
    },
    render::WindowId,
    state::{DocumentId, DocumentStoreTypes, State, ViewId, ViewStoreTypes},
    types::{Pos, RectSplit},
};

pub enum ViewCommand {
    Init { window: WindowId, view: ViewId, doc: DocumentId },
    ScrollTo { view: ViewId, pos: Pos },
    ScrollIfNeeded { view: ViewId, pos: Pos },
    SpawnWindow { doc: DocumentId, split: RectSplit },
    Resize,
}

pub struct ViewProtocol {
    local_store: Arc<RwLock<LocalViewStore>>,
    state: State,

    rx: UnboundedReceiver<ViewCommand>,
}

impl ViewProtocol {
    pub fn new(state: State, rx: UnboundedReceiver<ViewCommand>) -> Self {
        Self { local_store: Arc::new(RwLock::new(HashMap::new())), state, rx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ViewCommand::Init { window, view, doc } => self.init(window, view, doc).await,
                ViewCommand::ScrollTo { view, pos } => self.scroll_to(view, pos).await,
                ViewCommand::ScrollIfNeeded { view, pos } => self.scroll_if_needed(view, pos).await,
                ViewCommand::SpawnWindow { doc, split } => self.spawn_window(doc, split).await,
                ViewCommand::Resize => self.resize().await,
            }
        }
    }

    async fn init(&mut self, window: WindowId, view: ViewId, doc: DocumentId) {
        let Some(height) = self.state.with_workspace(|w| w.get_rect(window).map(|r| r.height))
        else {
            return;
        };
        let layout = self
            .state
            .with_view(view, |vm| vm.get::<ViewStoreTypes::Layout>().cloned())
            .flatten()
            .unwrap_or_default();

        let buffer_height = height.saturating_sub(layout.mode_line);
        if buffer_height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(Pos::default()), buffer_height).await;
        }
    }

    async fn scroll_to(&mut self, view: ViewId, pos: Pos) {
        let Some(rect) = self
            .state
            .with_window_view_map(|wv| wv.iter().find(|&(_, &v)| v == view).map(|(&win, _)| win))
            .flatten()
            .and_then(|win| self.state.with_workspace(|w| w.get_rect(win)))
        else {
            return;
        };
        let Some((Some(doc), layout)) = self.state.with_view(view, |vm| {
            let doc = vm.get::<DocumentId>().cloned();
            let layout = vm.get::<ViewStoreTypes::Layout>().cloned().unwrap_or_default();

            (doc, layout)
        }) else {
            return;
        };

        let buffer_height = rect.height.saturating_sub(layout.mode_line);
        if buffer_height == 0 {
            return;
        }

        let scroll = ViewStoreTypes::Scroll(pos);

        self.state.with_view_mut(view, |vm| vm.insert(scroll));
        self.fetch(view, doc, scroll, buffer_height).await;
    }

    async fn scroll_if_needed(&mut self, view: ViewId, pos: Pos) {
        let Some(rect) = self
            .state
            .with_window_view_map(|wv| wv.iter().find(|&(_, &v)| v == view).map(|(&win, _)| win))
            .flatten()
            .and_then(|win| self.state.with_workspace(|w| w.get_rect(win)))
        else {
            return;
        };
        let Some((Some(doc), mut scroll, layout)) = self.state.with_view(view, |vm| {
            let doc = vm.get::<DocumentId>().cloned();
            let scroll = vm.get::<ViewStoreTypes::Scroll>().cloned().unwrap_or_default();
            let layout = vm.get::<ViewStoreTypes::Layout>().cloned().unwrap_or_default();

            (doc, scroll, layout)
        }) else {
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
            self.state.with_view_mut(view, |vm| vm.insert(scroll));

            self.fetch(view, doc, scroll, buffer_height).await;
        }
    }

    async fn spawn_window(&mut self, doc: DocumentId, split: RectSplit) {
        let view = self.state.create_view(doc);

        let view_renderer =
            ViewRenderer::new(doc, view, self.local_store.clone(), self.state.clone());
        let view_decorator_renderer =
            Box::new(ViewDecoratorRenderer::new(view, view_renderer, self.state.clone()));

        let window =
            self.state.with_workspace_mut(|w| w.create_tile(view_decorator_renderer, split));
        self.state.with_window_view_map_mut(|mut wv| wv.insert(window, view));

        self.init(window, view, doc).await;
    }

    async fn resize(&mut self) {
        let mut entries = Vec::new();

        let mappings = self
            .state
            .with_window_view_map(|wv| wv.iter().map(|(&w, &v)| (w, v)).collect::<Vec<_>>())
            .unwrap_or_default();
        for (window, view) in mappings {
            let Some((Some(doc), scroll, layout)) = self.state.with_view(view, |vm| {
                let doc = vm.get::<DocumentId>().cloned();
                let scroll = vm.get::<ViewStoreTypes::Scroll>().cloned().unwrap_or_default();
                let layout = vm.get::<ViewStoreTypes::Layout>().cloned().unwrap_or_default();

                (doc, scroll, layout)
            }) else {
                continue;
            };
            let Some(rect) = self.state.with_workspace(|w| w.get_rect(window)) else {
                continue;
            };
            let buffer_height = rect.height.saturating_sub(layout.mode_line);

            entries.push((view, doc, scroll, buffer_height));
        }

        for (view, doc, scroll, height) in entries {
            if height > 0 {
                self.fetch(view, doc, scroll, height).await;
            }
        }
    }

    async fn fetch(
        &self, view: ViewId, doc: DocumentId, scroll: ViewStoreTypes::Scroll, height: usize,
    ) {
        let Some(lines) = self
            .state
            .with_doc(doc, |dm| {
                dm.get::<DocumentStoreTypes::Document>().and_then(|d| Some(d.data.lines()))
            })
            .flatten()
        else {
            return;
        };

        let digits = lines.checked_ilog10().unwrap_or(0) as usize + 1;
        let gutter = digits + 2;

        self.state.with_view_mut(view, |vm| {
            let mut layout = vm.get::<ViewStoreTypes::Layout>().cloned().unwrap_or_default();
            if layout.gutter != gutter {
                layout.gutter = gutter;
                vm.insert(layout);
            }
        });

        let Some((offset, data)) = self
            .state
            .with_doc(doc, |dm| {
                dm.get::<DocumentStoreTypes::Document>().and_then(|d| {
                    let start = d.data.get_line_start_byte(scroll.y);
                    let end = d.data.get_line_end_byte(scroll.y + height);

                    Some((start, d.data.slice(start..end)))
                })
            })
            .flatten()
        else {
            return;
        };

        let mut lines = data.split_inclusive('\n').map(String::from).collect::<Vec<_>>();
        if data.ends_with('\n') {
            lines.push(String::new());
        }

        self.local_store.write().unwrap().insert(view, LocalViewData { offset, lines });
    }
}
