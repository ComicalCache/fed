use piece_table::Slice;
use tokio::sync::{
    broadcast,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    debug_panic::debug_panic,
    protocols::{screen::ScreenCommand, view::ViewCommand},
    render::WindowId,
    state::{DocId, DocStoreTypes, State, StateLock, ViewId, ViewStoreTypes},
    types::Pos,
    util,
};

pub struct ViewProtocol {
    state_lock: StateLock,

    rx: UnboundedReceiver<ViewCommand>,
    doc_event_rx: broadcast::Receiver<DocStoreTypes::Event>,
    view_event_rx: broadcast::Receiver<ViewStoreTypes::Event>,
    screen_tx: UnboundedSender<ScreenCommand>,
}

impl ViewProtocol {
    pub fn new(
        state_lock: StateLock, rx: UnboundedReceiver<ViewCommand>,
        doc_event_rx: broadcast::Receiver<DocStoreTypes::Event>,
        view_event_rx: broadcast::Receiver<ViewStoreTypes::Event>,
        screen_tx: UnboundedSender<ScreenCommand>,
    ) -> Self {
        Self { state_lock, rx, doc_event_rx, view_event_rx, screen_tx }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                cmd = self.rx.recv() => {
                    let Some(cmd) = cmd else { break };
                    self.command(cmd);
                }
                event = self.doc_event_rx.recv() => {
                    let Ok(event) = event else { continue };
                    self.doc_event(event);
                }
                event = self.view_event_rx.recv() => {
                    let Ok(event) = event else { continue };
                    self.view_event(event);
                }
            }

            // Always redraw the screen after any view command or event.
            let _ = self.screen_tx.send(ScreenCommand::Render);
        }
    }

    fn command(&mut self, cmd: ViewCommand) {
        match cmd {
            ViewCommand::Update { view } => self.update(view),
            ViewCommand::ScrollTo { view, pos } => self.scroll_to(view, pos),
            ViewCommand::Resize => self.resize(),
        }
    }

    fn doc_event(&mut self, event: DocStoreTypes::Event) {
        let doc = match event {
            DocStoreTypes::Event::Created { .. } => return,
            DocStoreTypes::Event::Destroyed { .. } => return,
            DocStoreTypes::Event::Inserted { id, .. } => id,
            DocStoreTypes::Event::Removed { id, .. } => id,
            DocStoreTypes::Event::Written { .. } => return,
            DocStoreTypes::Event::ModeChanged { .. } => return,
        };

        let state = self.state_lock.read();
        let views: Vec<_> = state
            .index
            .doc_to_views(doc)
            .map(|views| views.iter().copied().collect())
            .unwrap_or_default();
        drop(state);

        for view in views {
            self.update(view);
        }
    }

    fn view_event(&mut self, event: ViewStoreTypes::Event) {
        match event {
            ViewStoreTypes::Event::CursorMoved { view } => self.cursor_moved(view),
            ViewStoreTypes::Event::CursorsChanged { .. } => {}
            ViewStoreTypes::Event::ModeChanged { .. } => {}
        }
    }

    fn update(&self, view: ViewId) {
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
        let height = windows
            .iter()
            .filter_map(|&w| state.workspace.get_rect(w))
            .map(|r| r.height.saturating_sub(vse.layout.mode_line))
            .max()
            .unwrap_or(0);
        drop(state);

        if height > 0 {
            self.fetch(view, doc, scroll, height);
        }
    }

    fn scroll_to(&self, view: ViewId, pos: Pos) {
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

        let height = windows
            .iter()
            .filter_map(|&w| state.workspace.get_rect(w))
            .map(|r| r.height.saturating_sub(vse.layout.mode_line))
            .max()
            .unwrap_or(0);

        vse.scroll = ViewStoreTypes::Scroll(pos);
        drop(guard);

        if height > 0 {
            self.fetch(view, doc, ViewStoreTypes::Scroll(pos), height);
        }
    }

    fn resize(&self) {
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
            let height = rect.height.saturating_sub(vse.layout.mode_line);

            entries.push((view, doc, scroll, height));
        }

        drop(state);

        for (view, doc, scroll, height) in entries {
            if height > 0 {
                self.fetch(view, doc, scroll, height);
            }
        }
    }

    fn cursor_moved(&self, view: ViewId) {
        let state = self.state_lock.read();
        let Some(window) = state.workspace.active_window else { return };

        if state.index.window_to_view(window) != Some(view) {
            return;
        }

        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(cursor) = vse.cursors.list.first() else {
            debug_panic!();
            return;
        };

        let pos = util::offset_to_pos(cursor.offset, vse, dse);
        drop(state);

        self.scroll_if_needed(window, view, pos);
    }

    fn fetch(&self, view: ViewId, doc: DocId, scroll: ViewStoreTypes::Scroll, height: usize) {
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

        let mut lines: Vec<_> = data.split_inclusive('\n').map(String::from).collect();
        if data.is_empty() || data.ends_with('\n') {
            lines.push(String::new());
        }

        let mut state = self.state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.view_cache = ViewStoreTypes::ViewCache { offset: start, lines };
        drop(state);
    }

    fn scroll_if_needed(&self, window: WindowId, view: ViewId, pos: Pos) {
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
        let max_height = windows
            .iter()
            .filter_map(|&w| state.workspace.get_rect(w))
            .map(|r| r.height.saturating_sub(vse.layout.mode_line))
            .max()
            .unwrap_or(0);

        let width = rect.width.saturating_sub(vse.layout.gutter_width(lines));
        let height = rect.height.saturating_sub(vse.layout.mode_line);
        if width == 0 || height == 0 {
            return;
        }
        drop(state);

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
}
