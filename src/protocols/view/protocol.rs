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
    debug_panic::debug_panic,
    protocols::{
        screen::ScreenCommand,
        view::{
            ViewRenderer,
            store::{LocalViewData, LocalViewStore},
        },
        view_decorator::ViewDecoratorRenderer,
    },
    render::{WindowId, ZLayer},
    state::{DocumentId, State, StateLock, ViewId, ViewStoreTypes},
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
        window: WindowId,
        view: ViewId,
        pos: Pos,
    },

    CreateRawTile {
        doc: DocumentId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
        tx: oneshot::Sender<(ViewId, WindowId)>,
    },
    CreateTile {
        doc: DocumentId,
        view: Option<ViewId>,
        split_window: WindowId,
        direction: RectSplit,
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
                ViewCommand::Init { window, view, doc } => self.init(window, view, doc),
                ViewCommand::Update { view } => self.update(view),
                ViewCommand::ScrollTo { view, pos } => self.scroll_to(view, pos),
                ViewCommand::ScrollIfNeeded { window, view, pos } => {
                    self.scroll_if_needed(window, view, pos)
                }

                ViewCommand::CreateRawTile { doc, view, split_window, direction, tx } => {
                    self.create_tile(doc, view, split_window, direction, tx, true)
                }
                ViewCommand::CreateTile { doc, view, split_window, direction, tx } => {
                    self.create_tile(doc, view, split_window, direction, tx, false)
                }
                ViewCommand::CreateRawFloating { doc, view, rect, z, tx } => {
                    self.create_floating(doc, view, rect, z, tx, true)
                }
                ViewCommand::CreateFloating { doc, view, rect, z, tx } => {
                    self.create_floating(doc, view, rect, z, tx, false)
                }
                ViewCommand::DestroyView { view } => self.destroy_view(view),

                ViewCommand::Resize => self.resize(),
            }

            // Always redraw the screen after any view command.
            let _ = self.screen_tx.send(ScreenCommand::Render);
        }
    }

    fn init(&mut self, window: WindowId, view: ViewId, doc: DocumentId) {
        let state = self.state_lock.read();
        debug_assert!(state.index.window_to_view(window) == Some(view));
        debug_assert!(state.index.view_to_doc(view) == Some(doc));

        let Some(height) = state.workspace.get_rect(window).map(|r| r.height) else {
            debug_panic!();
            return;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };

        let layout = vse.layout;
        drop(state);

        let height = height.saturating_sub(layout.mode_line);
        if height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(Pos::default()), height);
        }
    }

    fn update(&mut self, view: ViewId) {
        let state = self.state_lock.read();
        let Some(windows) = state.index.view_to_windows(view) else { return };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        let scroll = vse.scroll;
        let layout = vse.layout;
        let height = windows
            .iter()
            .map(|&w| {
                let rect = state.workspace.get_rect(w);
                debug_assert!(rect.is_some());
                rect
            })
            .flatten()
            .map(|r| r.height.saturating_sub(layout.mode_line))
            .max()
            .unwrap_or(0);
        drop(state);

        if height > 0 {
            self.fetch(view, doc, scroll, height);
        }
    }

    fn scroll_to(&mut self, view: ViewId, pos: Pos) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(windows) = state.index.view_to_windows(view) else { return };
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        let layout = vse.layout;
        let height = windows
            .iter()
            .map(|&w| {
                let rect = state.workspace.get_rect(w);
                debug_assert!(rect.is_some());
                rect
            })
            .flatten()
            .map(|r| r.height.saturating_sub(layout.mode_line))
            .max()
            .unwrap_or(0);

        vse.scroll = ViewStoreTypes::Scroll(pos);
        drop(guard);

        if height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(pos), height);
        }
    }

    fn scroll_if_needed(&mut self, window: WindowId, view: ViewId, pos: Pos) {
        let state = self.state_lock.read();
        let Some(windows) = state.index.view_to_windows(view) else { return };

        if !windows.contains(&window) {
            debug_panic!();
            return;
        }

        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        let Some(rect) = state.workspace.get_rect(window) else {
            debug_panic!();
            return;
        };

        let lines = dse.doc.data.lines();
        let mut scroll = vse.scroll;
        let layout = vse.layout;
        let max_height = windows
            .iter()
            .map(|&w| {
                let rect = state.workspace.get_rect(w);
                debug_assert!(rect.is_some());
                rect
            })
            .flatten()
            .map(|r| r.height.saturating_sub(layout.mode_line))
            .max()
            .unwrap_or(0);
        drop(state);

        let width = rect.width.saturating_sub(layout.gutter_width(lines));
        let height = rect.height.saturating_sub(layout.mode_line);
        if width == 0 || height == 0 {
            return;
        }

        let mut scroll_needed = false;

        if pos.y < scroll.y {
            scroll.y = pos.y;
            scroll_needed = true;
        } else if pos.y >= scroll.y + height {
            scroll.y = pos.y.saturating_sub(height).saturating_add(1);
            scroll_needed = true;
        }

        if pos.x < scroll.x {
            scroll.x = pos.x;
            scroll_needed = true;
        } else if pos.x >= scroll.x + width {
            scroll.x = pos.x.saturating_sub(width).saturating_add(1);
            scroll_needed = true;
        }

        if scroll_needed {
            let mut state = self.state_lock.write();
            let Some(vse) = state.view_store.get_mut(&view) else {
                debug_panic!();
                return;
            };

            vse.scroll = scroll;
            drop(state);

            self.fetch(view, doc, scroll, max_height);
        }
    }

    fn create_tile(
        &mut self, doc: DocumentId, view: Option<ViewId>, split_window: WindowId,
        direction: RectSplit, tx: oneshot::Sender<(ViewId, WindowId)>, raw: bool,
    ) {
        let mut state = self.state_lock.write();
        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window = if raw {
            let renderer = Box::new(ViewRenderer::new(doc, view, self.local_store.clone()));

            state.workspace.create_tile(split_window, direction, renderer)
        } else {
            let renderer = Box::new(ViewDecoratorRenderer::new(
                view,
                ViewRenderer::new(doc, view, self.local_store.clone()),
            ));

            state.workspace.create_tile(split_window, direction, renderer)
        };

        let Some(window) = window else {
            let windows = state.destroy_view(view);
            debug_assert!(windows.is_empty());

            debug_panic!();

            return;
        };

        state.index.link_window_to_view(window, view);
        drop(state);

        self.init(window, view, doc);

        let _ = tx.send((view, window));
    }

    fn create_floating(
        &mut self, doc: DocumentId, view: Option<ViewId>, rect: Rect, z: ZLayer,
        tx: oneshot::Sender<(ViewId, WindowId)>, raw: bool,
    ) {
        let mut state = self.state_lock.write();
        let view = view.unwrap_or_else(|| state.create_view(doc));

        let window = if raw {
            let renderer = Box::new(ViewRenderer::new(doc, view, self.local_store.clone()));

            state.workspace.create_floating(rect, z, renderer)
        } else {
            let renderer = Box::new(ViewDecoratorRenderer::new(
                view,
                ViewRenderer::new(doc, view, self.local_store.clone()),
            ));

            state.workspace.create_floating(rect, z, renderer)
        };
        state.index.link_window_to_view(window, view);
        drop(state);

        self.init(window, view, doc);

        let _ = tx.send((view, window));
    }

    fn destroy_view(&mut self, view: ViewId) {
        let mut state = self.state_lock.write();
        let mut local_store = self.local_store.write().unwrap();

        for window in state.index.unlink_view(view) {
            state.workspace.destroy_window(window);
        }

        local_store.remove(&view);

        drop(local_store);
        drop(state);
    }

    fn resize(&mut self) {
        let state = self.state_lock.read();

        let mappings: Vec<_> = state.index.window_to_view.iter().map(|(&w, &v)| (w, v)).collect();

        let mut entries = Vec::new();
        for (window, view) in mappings {
            let Some(rect) = state.workspace.get_rect(window) else {
                debug_panic!();
                continue;
            };
            let Some(vse) = state.view_store.get(&view) else {
                debug_panic!();
                continue;
            };
            let Some(doc) = state.index.view_to_doc(view) else {
                debug_panic!();
                continue;
            };

            let scroll = vse.scroll;
            let layout = vse.layout;
            let height = rect.height.saturating_sub(layout.mode_line);

            entries.push((view, doc, scroll, height));
        }

        drop(state);

        for (view, doc, scroll, height) in entries {
            if height > 0 {
                self.fetch(view, doc, scroll, height);
            }
        }
    }

    fn fetch(&self, view: ViewId, doc: DocumentId, scroll: ViewStoreTypes::Scroll, height: usize) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

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
