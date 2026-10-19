use piece_table::Slice;

use crate::{
    protocols::view::ViewCmd,
    state::{DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{Pos, Rect},
    util,
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

        for view in s.index.doc_to_views(*doc).cloned().unwrap_or_default() {
            Self::update(s, view);
        }
    }

    pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
        match event {
            ViewStoreTypes::Event::Created { id } | ViewStoreTypes::Event::Resized { id } => {
                Self::update(s, *id)
            }
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
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();

        let start = dse.doc.data.get_line_start_byte(vse.scroll.y);
        let end = dse.doc.data.get_line_end_byte(vse.scroll.y + vse.layout(dse).content.height);

        vse.decs.update(start, end);
        dse.decs.update(start, end);

        let data = dse.doc.data.slice(start..end);

        let mut lines: Vec<_> = data.split_inclusive('\n').map(String::from).collect();
        if data.is_empty() || data.ends_with('\n') {
            lines.push(String::new());
        }

        vse.view_cache = ViewStoreTypes::ViewCache { offset: start, lines };
    }

    pub fn scroll_to(s: &mut State, view: ViewId, pos: Pos) {
        let vse = s.view_store.get_mut(&view).unwrap();

        vse.scroll = ViewStoreTypes::Scroll(pos);

        Self::update(s, view);
    }

    pub fn resize(s: &mut State) {
        let mut windows = s.workspace.tiles();
        windows.extend(s.workspace.floatings());

        for window in windows {
            let views = s.index.window_to_views(window).unwrap();

            // Resizing for windows with multiple views must be done by the
            // entity that manages that window composition.
            if views.len() != 1 {
                continue;
            }

            let rect = s.workspace.get_rect(window).unwrap();

            let view = views[0];
            let vse = s.view_store.get_mut(&view).unwrap();

            vse.rect = Rect::new(Pos::default(), rect.width, rect.height);

            Self::update(s, view);
        }
    }

    fn cursor_moved(s: &mut State, view: ViewId) {
        let Some(window) = s.workspace.active_window else { return };

        // Don't scroll windows that don't have focus.
        if !s.index.window_to_views(window).is_some_and(|views| views.contains(&view)) {
            return;
        }

        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();
        let cursor = vse.cursors.list.first().unwrap();

        let content = vse.layout(dse).content;
        if content.width == 0 || content.height == 0 {
            return;
        }

        let mut scroll = vse.scroll;
        let mut scroll_needed = false;

        let pos = util::offset_to_pos(cursor.offset, vse, dse);
        if pos.y < scroll.y {
            scroll.y = pos.y;
            scroll_needed = true;
        } else if pos.y >= scroll.y + content.height {
            scroll.y = pos.y.saturating_sub(content.height).saturating_add(1);
            scroll_needed = true;
        }

        if pos.x < scroll.x {
            scroll.x = pos.x;
            scroll_needed = true;
        } else if pos.x >= scroll.x + content.width {
            scroll.x = pos.x.saturating_sub(content.width).saturating_add(1);
            scroll_needed = true;
        }

        if scroll_needed {
            let vse = s.view_store.get_mut(&view).unwrap();

            vse.scroll = scroll;

            Self::update(s, view);
        }
    }
}
