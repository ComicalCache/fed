use crate::piece_table::Chunks;
use crate::protocols::view::ViewCmd;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Pos;
use crate::types::Rect;
use crate::util;

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
            | DocStoreTypes::Event::Written { .. } => return,
            DocStoreTypes::Event::Inserted { doc, .. }
            | DocStoreTypes::Event::Removed { doc, .. }
            | DocStoreTypes::Event::ModeChanged { doc, .. }
            | DocStoreTypes::Event::PathChanged { doc, .. } => doc,
        };

        // Event may be received after the doc has already been removed again.
        if !s.doc_store.contains_key(doc) {
            return;
        }

        for view in s.index.doc_to_views(*doc).cloned().unwrap_or_default() {
            Self::update(s, view);
        }
    }

    pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
        let view = match event {
            ViewStoreTypes::Event::Created { view }
            | ViewStoreTypes::Event::Resized { view }
            | ViewStoreTypes::Event::CursorsMoved { view } => *view,
            ViewStoreTypes::Event::CursorsChanged { .. }
            | ViewStoreTypes::Event::ModeChanged { .. }
            | ViewStoreTypes::Event::Destroyed { .. } => return,
        };

        // Event may be received after the doc has already been removed again.
        if !s.view_store.contains_key(&view) {
            return;
        }

        if matches!(event, ViewStoreTypes::Event::CursorsMoved { .. }) {
            Self::cursor_moved(s, view);
        } else {
            Self::update(s, view);
        }
    }

    pub fn update(s: &mut State, view: ViewId) {
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();

        let start = dse.doc.data.line_start_offset(vse.scroll.y);
        let end = dse.doc.data.line_end_offset(vse.scroll.y + vse.layout(dse).content.height);

        vse.decs.update(start, end);
        dse.decs.update(start, end);

        // Avoid re-populating the cache when the contents didn't change.
        let revision = Some((dse.doc.revision, start, end));
        if vse.view_cache.revision == revision {
            return;
        }

        let mut lines = Vec::new();
        let mut line = String::new();
        for chunk in dse.doc.data.chunks(start..end) {
            for part in chunk.split_inclusive('\n') {
                line.push_str(part);

                if part.ends_with('\n') {
                    lines.push(std::mem::take(&mut line));
                }
            }
        }
        lines.push(line);

        vse.view_cache = ViewStoreTypes::ViewCache { offset: start, lines, revision };
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

        Self::scroll_if_needed(s, view);
    }

    pub fn scroll_if_needed(s: &mut State, view: ViewId) {
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
