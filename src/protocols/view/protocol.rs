use piece_table::Slice;

use crate::{
    debug_panic::debug_panic,
    protocols::{screen::ScreenProtocol, state::PState, view::ViewCmd},
    render::WindowId,
    state::{DocId, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::Pos,
    util,
};

pub struct ViewProtocol {}

impl ViewProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState, cmd: ViewCmd) {
        match cmd {
            ViewCmd::Update { view } => Self::update(state, pstate, view),
            ViewCmd::ScrollTo { view, pos } => Self::scroll_to(state, view, pos),
            ViewCmd::Resize => Self::resize(state),
        }

        // Always redraw the screen after any view command or event.
        ScreenProtocol::render(pstate);
    }

    pub fn exec_doc_event(state: &mut State, pstate: &mut PState, event: DocStoreTypes::Event) {
        let doc = match event {
            DocStoreTypes::Event::Created { .. } => return,
            DocStoreTypes::Event::Destroyed { .. } => return,
            DocStoreTypes::Event::Inserted { id, .. } => id,
            DocStoreTypes::Event::Removed { id, .. } => id,
            DocStoreTypes::Event::Written { .. } => return,
            DocStoreTypes::Event::ModeChanged { .. } => return,
            DocStoreTypes::Event::PathChanged { id, .. } => id,
        };

        let views: Vec<_> = state
            .index
            .doc_to_views(doc)
            .map(|views| views.iter().copied().collect())
            .unwrap_or_default();
        for view in views {
            Self::update(state, pstate, view);
        }

        // Always redraw the screen after any view command or event.
        ScreenProtocol::render(pstate);
    }

    pub fn exec_view_event(state: &mut State, pstate: &mut PState, event: ViewStoreTypes::Event) {
        match event {
            ViewStoreTypes::Event::CursorMoved { view } => Self::cursor_moved(state, view),
            ViewStoreTypes::Event::CursorsChanged { .. } => {}
            ViewStoreTypes::Event::ModeChanged { .. } => {}
        }

        // Always redraw the screen after any view command or event.
        ScreenProtocol::render(pstate);
    }

    pub fn update(state: &mut State, pstate: &mut PState, view: ViewId) {
        let Some(windows) = state.index.view_to_windows(view) else { return };
        let Some(vse) = state.view_store.get(&view) else {
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

        if height > 0 {
            Self::fetch(state, view, doc, vse.scroll, height);
        }

        ScreenProtocol::render(pstate);
    }

    pub fn scroll_to(state: &mut State, view: ViewId, pos: Pos) {
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

        if height > 0 {
            Self::fetch(state, view, doc, ViewStoreTypes::Scroll(pos), height);
        }
    }

    pub fn resize(state: &mut State) {
        let mappings: Vec<_> = state.index.window_to_view.iter().map(|(&w, &v)| (w, v)).collect();

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

            let height = rect.height.saturating_sub(vse.layout.mode_line);
            if height > 0 {
                Self::fetch(state, view, doc, vse.scroll, height);
            }
        }
    }

    fn cursor_moved(state: &mut State, view: ViewId) {
        let Some(window) = state.workspace.active_window else { return };

        // Don't scroll windows that don't have focus.
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

        Self::scroll_if_needed(state, window, view, pos);
    }

    fn fetch(
        state: &mut State, view: ViewId, doc: DocId, scroll: ViewStoreTypes::Scroll, height: usize,
    ) {
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

        let mut lines: Vec<_> = data.split_inclusive('\n').map(String::from).collect();
        if data.is_empty() || data.ends_with('\n') {
            lines.push(String::new());
        }

        vse.view_cache = ViewStoreTypes::ViewCache { offset: start, lines };
    }

    fn scroll_if_needed(state: &mut State, window: WindowId, view: ViewId, pos: Pos) {
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

        let mut scroll = vse.scroll;
        let max_height = windows
            .iter()
            .filter_map(|&w| state.workspace.get_rect(w))
            .map(|r| r.height.saturating_sub(vse.layout.mode_line))
            .max()
            .unwrap_or(0);

        let width = rect.width.saturating_sub(vse.layout.gutter_width(dse.doc.data.lines()));
        let height = rect.height.saturating_sub(vse.layout.mode_line);
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
            let Some(vse) = state.view_store.get_mut(&view) else {
                debug_panic!();
                return;
            };

            vse.scroll = scroll;

            Self::fetch(state, view, doc, scroll, max_height);
        }
    }
}
