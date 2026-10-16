use piece_table::Slice;

use crate::{
    protocols::view::ViewCmd,
    state::{DocId, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{Pos, render::WindowId},
    util::{self, debug_panic},
};

pub struct ViewProtocol {}

impl ViewProtocol {
    pub fn exec(s: &mut State, cmd: ViewCmd) {
        match cmd {
            ViewCmd::Update { view } => Self::update(s, view),
            ViewCmd::ScrollTo { view, pos } => Self::scroll_to(s, view, pos),

            ViewCmd::Resize => Self::resize(s),
        }
    }

    pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
        let doc = match event {
            DocStoreTypes::Event::Created { .. }
            | DocStoreTypes::Event::Destroyed { .. }
            | DocStoreTypes::Event::Written { .. }
            | DocStoreTypes::Event::ModeChanged { .. } => return,
            DocStoreTypes::Event::Inserted { id, .. }
            | DocStoreTypes::Event::Removed { id, .. }
            | DocStoreTypes::Event::PathChanged { id, .. } => id,
        };

        let views: Vec<_> =
            s.index.doc_to_views(*doc).map_or_default(|views| views.iter().copied().collect());
        for view in views {
            Self::update(s, view);
        }
    }

    pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
        match event {
            ViewStoreTypes::Event::Created { id } => Self::update(s, *id),
            ViewStoreTypes::Event::CursorsMoved { view } => Self::cursor_moved(s, *view),
            ViewStoreTypes::Event::CursorsChanged { .. }
            | ViewStoreTypes::Event::ModeChanged { .. }
            | ViewStoreTypes::Event::Destroyed { .. } => {
                // The view cache gets deleted when the vse gets deleted,
                // nothing needs to be done.
            }
        }
    }

    pub fn update(s: &mut State, view: ViewId) {
        let Some(windows) = s.index.view_to_windows(view) else { return };
        let Some(vse) = s.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(doc) = s.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        let height = windows
            .iter()
            .filter_map(|&w| s.workspace.get_rect(w))
            .map(|r| r.height.saturating_sub(vse.layout.mode_line))
            .max()
            .unwrap_or(0);

        if height > 0 {
            Self::fetch(s, view, doc, vse.scroll, height);
        }
    }

    pub fn scroll_to(s: &mut State, view: ViewId, pos: Pos) {
        let Some(windows) = s.index.view_to_windows(view) else { return };
        let Some(vse) = s.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(doc) = s.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        let height = windows
            .iter()
            .filter_map(|&w| s.workspace.get_rect(w))
            .map(|r| r.height.saturating_sub(vse.layout.mode_line))
            .max()
            .unwrap_or(0);

        vse.scroll = ViewStoreTypes::Scroll(pos);

        if height > 0 {
            Self::fetch(s, view, doc, ViewStoreTypes::Scroll(pos), height);
        }
    }

    pub fn resize(s: &mut State) {
        let mappings: Vec<_> = s.index.window_to_view.iter().map(|(&w, &v)| (w, v)).collect();

        for (window, view) in mappings {
            let Some(rect) = s.workspace.get_rect(window) else {
                debug_panic!();
                continue;
            };
            let Some(vse) = s.view_store.get(&view) else {
                debug_panic!();
                continue;
            };
            let Some(doc) = s.index.view_to_doc(view) else {
                debug_panic!();
                continue;
            };

            let height = rect.height.saturating_sub(vse.layout.mode_line);
            if height > 0 {
                Self::fetch(s, view, doc, vse.scroll, height);
            }
        }
    }

    fn cursor_moved(s: &mut State, view: ViewId) {
        let Some(window) = s.workspace.active_window else { return };

        // Don't scroll windows that don't have focus.
        if s.index.window_to_view(window) != Some(view) {
            return;
        }

        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(cursor) = vse.cursors.list.first() else {
            debug_panic!();
            return;
        };

        let pos = util::offset_to_pos(cursor.offset, vse, dse);

        Self::scroll_if_needed(s, window, view, pos);
    }

    fn fetch(
        s: &mut State, view: ViewId, doc: DocId, scroll: ViewStoreTypes::Scroll, height: usize,
    ) {
        let Some(vse) = s.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = s.doc_store.get_mut(&doc) else {
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

    fn scroll_if_needed(s: &mut State, window: WindowId, view: ViewId, pos: Pos) {
        let Some(windows) = s.index.view_to_windows(view) else { return };

        if !windows.contains(&window) {
            debug_panic!();
            return;
        }

        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(doc) = s.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        let Some(rect) = s.workspace.get_rect(window) else {
            debug_panic!();
            return;
        };

        let mut scroll = vse.scroll;
        let max_height = windows
            .iter()
            .filter_map(|&w| s.workspace.get_rect(w))
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
            let Some(vse) = s.view_store.get_mut(&view) else {
                debug_panic!();
                return;
            };

            vse.scroll = scroll;

            Self::fetch(s, view, doc, scroll, max_height);
        }
    }
}
