use std::path::PathBuf;

use piece_table::{Kind, PieceTable, Slice};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    fed::FEvent,
    protocols::action::ActionCmd,
    state::{DocId, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{Cursor, Decorator, Motion, Pos, render::WindowId},
    util,
};

struct Edit {
    offset: usize,
    remove: usize,
    insert: String,
}

pub struct ActionProtocol {}

impl ActionProtocol {
    pub fn exec(s: &mut State, cmd: ActionCmd) {
        match cmd {
            ActionCmd::MoveCursors { view, motion: direction, move_anchor } => {
                Self::move_cursors(s, view, direction, move_anchor);
            }
            ActionCmd::MoveCursorToPos { view, pos, move_anchor } => {
                Self::move_cursor_to_pos(s, view, pos, move_anchor);
            }
            ActionCmd::CreateCursorAtPos { view, pos } => {
                Self::create_cursor_at_pos(s, view, pos);
            }
            ActionCmd::CreateCursorsAtOffset { view, offsets } => {
                Self::create_cursors_at_offset(s, view, offsets);
            }
            ActionCmd::RemoveCursors { view, offsets } => {
                Self::remove_cursors(s, view, &offsets);
            }
            ActionCmd::ResetCursorAnchors { view } => Self::reset_cursor_anchors(s, view),

            ActionCmd::StartCommit { doc } => Self::start_commit(s, doc),
            ActionCmd::EndCommit { doc } => Self::end_commit(s, doc),

            ActionCmd::Undo { view } => Self::undo(s, view),
            ActionCmd::HotRedo { view } => Self::hot_redo(s, view),

            ActionCmd::Saved { doc } => Self::saved(s, doc),

            ActionCmd::Insert { view, text } => Self::insert(s, view, text),
            ActionCmd::InsertAt { view, text, offset } => {
                Self::insert_at(s, view, text, offset);
            }
            ActionCmd::Backspace { view } => Self::backspace(s, view),
            ActionCmd::Delete { view } => Self::delete(s, view),
            ActionCmd::Remove { view, offset, len } => Self::remove(s, view, offset, len),

            ActionCmd::SetDocPath { doc, path } => Self::set_doc_path(s, doc, path),

            ActionCmd::SetDocMode { doc, mode } => Self::set_doc_mode(s, doc, mode),
            ActionCmd::PushViewMode { view, mode } => Self::push_view_mode(s, view, mode),
            ActionCmd::PopViewMode { view } => Self::pop_view_mode(s, view),

            ActionCmd::SetDocDecorator { doc, id, dec } => {
                Self::set_doc_decorator(s, doc, id, dec);
            }
            ActionCmd::SetViewDecorator { view, id, dec } => {
                Self::set_view_decorator(s, view, id, dec);
            }
            ActionCmd::RemoveDocDecorator { doc, id } => Self::remove_doc_decorator(s, doc, id),
            ActionCmd::RemoveViewDecorator { view, id } => {
                Self::remove_view_decorator(s, view, id);
            }

            ActionCmd::SetActiveWindow { window } => Self::set_active_window(s, window),
        }
    }

    pub fn move_cursors(s: &mut State, view: ViewId, motion: Motion, move_anchor: bool) {
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();

        let mut cursors = vse.cursors.clone();
        for cursor in &mut cursors.list {
            util::motion::apply(cursor, motion, vse, dse);

            if move_anchor {
                cursor.anchor = cursor.offset;
            }
        }
        vse.cursors = cursors.clone();

        vse.cursors.normalize();
        if vse.cursors.list.is_empty() {
            return;
        }

        let _ = s
            .fevent_tx
            .send(crate::fed::FEvent::View(ViewStoreTypes::Event::CursorsMoved { view }));
    }

    pub fn move_cursor_to_pos(s: &mut State, view: ViewId, pos: Pos, move_anchor: bool) {
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();

        let offset = util::pos_to_offset(pos, vse, dse);

        let anchor = if move_anchor {
            offset
        } else {
            vse.cursors.list.first().map_or(offset, |c| c.anchor)
        };

        vse.cursors.list.clear();

        let mut cursor = Cursor::new(offset, pos.x);
        cursor.anchor = anchor;
        vse.cursors.list.push(cursor);

        let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsMoved { view }));
    }

    pub fn create_cursor_at_pos(s: &mut State, view: ViewId, pos: Pos) {
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();

        if let Some(max_cursors) = dse.max_cursors
            && vse.cursors.list.len() >= max_cursors
        {
            return;
        }

        let offset = util::pos_to_offset(pos, vse, dse);

        vse.cursors.list.push(Cursor::new(offset, pos.x));
        vse.cursors.normalize();

        let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn create_cursors_at_offset(s: &mut State, view: ViewId, mut offsets: Vec<usize>) {
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();

        if let Some(max_cursors) = dse.max_cursors {
            if vse.cursors.list.len() >= max_cursors {
                return;
            }

            if vse.cursors.list.len() + offsets.len() > max_cursors {
                offsets.drain(..max_cursors - vse.cursors.list.len());
            }
        }

        for offset in offsets {
            vse.cursors.list.push(Cursor::new(offset, 0));
        }

        vse.cursors.normalize();

        let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsMoved { view }));
    }

    pub fn remove_cursors(s: &mut State, view: ViewId, offsets: &[usize]) {
        let vse = s.view_store.get_mut(&view).unwrap();

        vse.cursors.list.retain(|c| !offsets.contains(&c.offset));

        let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn reset_cursor_anchors(s: &mut State, view: ViewId) {
        let vse = s.view_store.get_mut(&view).unwrap();

        for cursor in &mut vse.cursors.list {
            cursor.anchor = cursor.offset;
        }

        let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn start_commit(s: &mut State, doc: DocId) {
        let dse = s.doc_store.get_mut(&doc).unwrap();

        assert!(!dse.read_only);

        dse.doc.data.start_commit();
    }

    pub fn end_commit(s: &mut State, doc: DocId) {
        let dse = s.doc_store.get_mut(&doc).unwrap();

        assert!(!dse.read_only);

        dse.doc.data.end_commit();
    }

    pub fn undo(s: &mut State, view: ViewId) {
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get_mut(&doc).unwrap();
        assert!(!dse.read_only);

        let edits = dse.doc.data.undo();
        Self::process_edits(s, doc, edits);
    }

    pub fn hot_redo(s: &mut State, view: ViewId) {
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get_mut(&doc).unwrap();
        assert!(!dse.read_only);

        let edits = dse.doc.data.hot_redo();
        Self::process_edits(s, doc, edits);
    }

    pub fn saved(s: &mut State, doc: DocId) {
        let dse = s.doc_store.get_mut(&doc).unwrap();

        if !dse.doc.modified {
            return;
        }

        dse.doc.modified = false;

        let path = dse.doc.path.clone();
        if let Some(path) = path {
            let _ = s.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Written {
                id: doc,
                path,
                bytes_written: dse.doc.data.len(),
            }));
        }
    }

    pub fn insert(s: &mut State, view: ViewId, text: String) {
        Self::execute_transaction(s, view, |_, cursors, _| {
            cursors
                .iter()
                .map(|cursor| Edit { offset: cursor.offset, remove: 0, insert: text.clone() })
                .collect()
        });
    }

    pub fn insert_at(s: &mut State, view: ViewId, text: String, offset: usize) {
        Self::execute_transaction(s, view, |_, _, _| {
            vec![Edit { offset, remove: 0, insert: text }]
        });
    }

    pub fn backspace(s: &mut State, view: ViewId) { Self::execute_remove(s, view, true); }

    pub fn delete(s: &mut State, view: ViewId) { Self::execute_remove(s, view, false); }

    pub fn remove(s: &mut State, view: ViewId, offset: usize, len: usize) {
        Self::execute_transaction(s, view, |_, _, _| {
            vec![Edit { offset, remove: len, insert: String::new() }]
        });
    }

    pub fn set_doc_path(s: &mut State, doc: DocId, path: Option<PathBuf>) {
        let dse = s.doc_store.get_mut(&doc).unwrap();
        dse.doc.path.clone_from(&path);

        let _ = s.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::PathChanged { id: doc, path }));
    }

    pub fn set_doc_mode(s: &mut State, doc: DocId, mode: DocStoreTypes::Mode) {
        let dse = s.doc_store.get_mut(&doc).unwrap();
        if dse.mode.singleton() || mode.singleton() {
            return;
        }

        dse.mode = mode;
        s.dmode(doc);

        let _ = s.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::ModeChanged { id: doc, mode }));
    }

    pub fn push_view_mode(s: &mut State, view: ViewId, mode: ViewStoreTypes::Mode) {
        let vse = s.view_store.get_mut(&view).unwrap();
        if vse.modes.last() != Some(&mode) {
            vse.modes.push(mode);
        }
        s.vmode(view);

        let _ =
            s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::ModeChanged { id: view, mode }));
    }

    pub fn pop_view_mode(s: &mut State, view: ViewId) {
        let Some(vse) = s.view_store.get_mut(&view) else {
            return;
        };

        if vse.modes.len() > 1 {
            vse.modes.pop();
        }

        let mode = vse.mode();
        s.vmode(view);

        let _ =
            s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::ModeChanged { id: view, mode }));
    }

    pub fn set_doc_decorator(
        s: &mut State, doc: DocId, id: DocStoreTypes::DecorationId, dec: Box<dyn Decorator>,
    ) {
        let dse = s.doc_store.get_mut(&doc).unwrap();
        dse.decs.decorators.insert(id, dec);
    }

    pub fn set_view_decorator(
        s: &mut State, view: ViewId, id: ViewStoreTypes::DecorationId, dec: Box<dyn Decorator>,
    ) {
        let vse = s.view_store.get_mut(&view).unwrap();
        vse.decs.decorators.insert(id, dec);
    }

    pub fn remove_doc_decorator(s: &mut State, doc: DocId, id: DocStoreTypes::DecorationId) {
        let dse = s.doc_store.get_mut(&doc).unwrap();
        dse.decs.decorators.remove(&id);
    }

    pub fn remove_view_decorator(s: &mut State, view: ViewId, id: ViewStoreTypes::DecorationId) {
        let vse = s.view_store.get_mut(&view).unwrap();
        vse.decs.decorators.remove(&id);
    }

    pub fn set_active_window(s: &mut State, window: Option<WindowId>) {
        assert!(window.is_none_or(|window| s.workspace.get_rect(window).is_some()));

        s.workspace.active_window = window;
    }

    pub fn can_quit(s: &State) -> Result<(), String> {
        for dse in s.doc_store.values() {
            if !dse.doc.modified {
                continue;
            }

            let name = dse
                .doc
                .path
                .as_ref()
                .map_or_else(|| "SCRATCHPAD".to_string(), |p| p.display().to_string());

            return Err(format!("'{name}' has unsaved changes"));
        }

        Ok(())
    }

    fn execute_remove(s: &mut State, view: ViewId, bksp: bool) {
        Self::execute_transaction(s, view, |doc_data, cursors, _| {
            let mut edits = Vec::new();
            for cursor in cursors {
                if bksp {
                    if cursor.offset == 0 {
                        continue;
                    }

                    let y = doc_data.get_line_of_byte(cursor.offset - 1);
                    let start = doc_data.get_line_start_byte(y);
                    let remove = if cursor.offset == start {
                        1
                    } else {
                        let text = doc_data.slice(start..cursor.offset);
                        text.graphemes(true).next_back().map_or(1, str::len)
                    };

                    edits.push(Edit {
                        offset: cursor.offset.saturating_sub(remove),
                        remove,
                        insert: String::new(),
                    });
                } else {
                    if cursor.offset >= doc_data.len() {
                        continue;
                    }

                    let y = doc_data.get_line_of_byte(cursor.offset);
                    let end = doc_data.get_line_end_byte(y);
                    let remove = if cursor.offset == end {
                        1
                    } else {
                        let text = doc_data.slice(cursor.offset..end);
                        text.graphemes(true).next().map_or(1, str::len)
                    };

                    edits.push(Edit { offset: cursor.offset, remove, insert: String::new() });
                }
            }

            edits
        });
    }

    fn execute_transaction(
        s: &mut State, view: ViewId,
        edits: impl FnOnce(&mut PieceTable, &Vec<Cursor>, ViewStoreTypes::TabWidth) -> Vec<Edit>,
    ) {
        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view).unwrap();
        let doc = s.index.view_to_doc(view).unwrap();

        assert!(!dse.read_only);

        let tab_width = vse.layout.tab_width;

        let mut edits = edits(&mut dse.doc.data, &vse.cursors.list, tab_width);
        edits.sort_by_key(|e| std::cmp::Reverse(e.offset));
        edits.dedup_by_key(|e| e.offset);

        let mut changes = Vec::new();
        for edit in edits {
            if edit.remove > 0 {
                let data = dse.doc.data.slice(edit.offset..edit.offset + edit.remove);
                dse.doc.data.remove(edit.offset, edit.remove);
                changes.push((edit.offset, Kind::Deletion, data));
            }
            if !edit.insert.is_empty() {
                dse.doc.data.insert(edit.offset, &edit.insert);
                changes.push((edit.offset, Kind::Insertion, edit.insert));
            }
        }

        Self::process_edits(s, doc, changes);
    }

    fn process_edits(s: &mut State, doc: DocId, edits: Vec<(usize, Kind, String)>) {
        if edits.is_empty() {
            return;
        }

        let dse = s.doc_store.get_mut(&doc).unwrap();
        dse.doc.modified = true;
        dse.doc.revision += 1;

        for (offset, kind, data) in &edits {
            let (remove, insert) = match kind {
                Kind::Deletion => (data.len(), 0),
                Kind::Insertion => (0, data.len()),
            };
            dse.decs.edit(*offset, remove, insert);
        }

        let views = s.index.doc_to_views(doc).cloned().unwrap_or_default();
        for &view in &views {
            let (vse, dse) =
                State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, view)
                    .unwrap();

            let mut cursors = vse.cursors.clone();
            for cursor in &mut cursors.list {
                let mut cursor_moved = false;
                for (offset, kind, data) in &edits {
                    cursor_moved |= cursor.offset >= *offset;

                    for position in [&mut cursor.offset, &mut cursor.anchor] {
                        if *position >= *offset {
                            match kind {
                                Kind::Deletion => {
                                    *position -= data.len().min(*position - *offset);
                                }
                                Kind::Insertion => *position += data.len(),
                            }
                        }
                    }
                }

                if cursor_moved {
                    cursor.pref_x = util::offset_to_pos(cursor.offset, vse, dse).x;
                }
            }
            cursors.normalize();

            assert!(
                cursors
                    .list
                    .iter()
                    .all(|c| c.offset <= dse.doc.data.len() && c.anchor <= dse.doc.data.len())
            );

            vse.cursors = cursors;

            for (offset, kind, data) in &edits {
                let (remove, insert) = match kind {
                    Kind::Deletion => (data.len(), 0),
                    Kind::Insertion => (0, data.len()),
                };
                vse.decs.edit(*offset, remove, insert);
            }
        }

        for (pos, kind, str) in edits {
            let event = match kind {
                Kind::Deletion => DocStoreTypes::Event::Removed { id: doc, pos, n: str.len(), str },
                Kind::Insertion => {
                    DocStoreTypes::Event::Inserted { id: doc, pos, n: str.len(), str }
                }
            };

            let _ = s.fevent_tx.send(FEvent::Doc(event));
        }

        for view in views {
            if s.view_store.get(&view).unwrap().cursors.list.is_empty() {
                continue;
            }

            let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsMoved { view }));
        }
    }
}
