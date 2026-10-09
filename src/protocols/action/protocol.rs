use std::path::PathBuf;

use piece_table::{Kind, PieceTable, Slice};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    fed::FEvent,
    protocols::{action::ActionCmd, state::PState},
    render::WindowId,
    state::{DocId, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{Cursor, Decorator, Motion, Pos},
    util,
};

struct Edit {
    offset: usize,
    remove: usize,
    insert: String,
}

pub struct ActionProtocol {}

impl ActionProtocol {
    pub fn exec(state: &mut State, cmd: ActionCmd) {
        match cmd {
            ActionCmd::MoveCursors { view, motion: direction, move_anchor } => {
                Self::move_cursors(state, view, direction, move_anchor);
            }
            ActionCmd::MoveCursorToPos { view, pos, move_anchor } => {
                Self::move_cursor_to_pos(state, view, pos, move_anchor);
            }
            ActionCmd::CreateCursorAtPos { view, pos } => {
                Self::create_cursor_at_pos(state, view, pos)
            }
            ActionCmd::CreateCursorsAtOffset { view, offsets } => {
                Self::create_cursors_at_offset(state, view, offsets)
            }
            ActionCmd::RemoveCursors { view, offsets } => {
                Self::remove_cursors(state, view, offsets)
            }
            ActionCmd::ResetCursorAnchors { view } => Self::reset_cursor_anchors(state, view),

            ActionCmd::StartCommit { doc } => Self::start_commit(state, doc),
            ActionCmd::EndCommit { doc } => Self::end_commit(state, doc),

            ActionCmd::Undo { view } => Self::undo(state, view),
            ActionCmd::HotRedo { view } => Self::hot_redo(state, view),

            ActionCmd::Saved { doc } => Self::saved(state, doc),

            ActionCmd::Insert { view, text } => Self::insert(state, view, text),
            ActionCmd::InsertAt { view, text, offset } => {
                Self::insert_at(state, view, text, offset)
            }
            ActionCmd::Backspace { view } => Self::backspace(state, view),
            ActionCmd::Delete { view } => Self::delete(state, view),
            ActionCmd::Remove { view, offset, len } => Self::remove(state, view, offset, len),

            ActionCmd::SetDocPath { doc, path } => Self::set_doc_path(state, doc, path),

            ActionCmd::SetDocMode { doc, mode } => Self::set_doc_mode(state, doc, mode),
            ActionCmd::PushViewMode { view, mode } => Self::push_view_mode(state, view, mode),
            ActionCmd::PopViewMode { view } => Self::pop_view_mode(state, view),

            ActionCmd::SetDocDecorator { doc, id, dec } => {
                Self::set_doc_decorator(state, doc, id, dec)
            }
            ActionCmd::SetViewDecorator { view, id, dec } => {
                Self::set_view_decorator(state, view, id, dec)
            }
            ActionCmd::RemoveDocDecorator { doc, id } => Self::remove_doc_decorator(state, doc, id),
            ActionCmd::RemoveViewDecorator { view, id } => {
                Self::remove_view_decorator(state, view, id)
            }

            ActionCmd::SetActiveWindow { window } => Self::set_active_window(state, window),
        }
    }

    pub fn move_cursors(state: &mut State, view: ViewId, motion: Motion, move_anchor: bool) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };

        let mut cursors = vse.cursors.clone();
        for cursor in &mut cursors.list {
            util::motion::apply(cursor, motion, &vse, &dse);

            if move_anchor {
                cursor.anchor = cursor.offset;
            }
        }
        vse.cursors = cursors.clone();

        vse.cursors.normalize();
        if vse.cursors.list.is_empty() {
            return;
        }

        let _ = state
            .fevent_tx
            .send(crate::fed::FEvent::View(ViewStoreTypes::Event::CursorMoved { view }));
    }

    pub fn move_cursor_to_pos(state: &mut State, view: ViewId, pos: Pos, move_anchor: bool) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };

        let offset = util::pos_to_offset(pos, vse, dse);

        let anchor = if move_anchor {
            offset
        } else {
            vse.cursors.list.first().map(|c| c.anchor).unwrap_or(offset)
        };

        vse.cursors.list.clear();

        let mut cursor = Cursor::new(offset, pos.x);
        cursor.anchor = anchor;
        vse.cursors.list.push(cursor);

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorMoved { view }));
    }

    pub fn create_cursor_at_pos(state: &mut State, view: ViewId, pos: Pos) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };

        if let Some(max_cursors) = dse.max_cursors
            && vse.cursors.list.len() >= max_cursors
        {
            return;
        }

        let offset = util::pos_to_offset(pos, vse, dse);

        vse.cursors.list.push(Cursor::new(offset, pos.x));
        vse.cursors.normalize();

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn create_cursors_at_offset(state: &mut State, view: ViewId, mut offsets: Vec<usize>) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };

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

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorMoved { view }));
    }

    pub fn remove_cursors(state: &mut State, view: ViewId, offsets: Vec<usize>) {
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.cursors.list.retain(|c| !offsets.contains(&c.offset));

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn reset_cursor_anchors(state: &mut State, view: ViewId) {
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        for cursor in &mut vse.cursors.list {
            cursor.anchor = cursor.offset;
        }

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn start_commit(state: &mut State, doc: DocId) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        if dse.read_only {
            debug_panic!();
            return;
        }

        dse.doc.data.start_commit();
    }

    pub fn end_commit(state: &mut State, doc: DocId) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        if dse.read_only {
            debug_panic!();
            return;
        }

        dse.doc.data.end_commit();
    }

    pub fn undo(state: &mut State, view: ViewId) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        if dse.read_only {
            debug_panic!();
            return;
        }

        let edits = dse.doc.data.undo();
        dse.doc.modified = true;

        let len = dse.doc.data.len();
        for cursor in &mut vse.cursors.list {
            cursor.offset = cursor.offset.min(len);
            cursor.anchor = cursor.anchor.min(len);
        }

        for (offset, kind, data) in &edits {
            let (remove, insert) = match kind {
                Kind::Deletion => (data.len(), 0),
                Kind::Insertion => (0, data.len()),
            };

            dse.decs.edit(*offset, remove, insert);
            vse.decs.edit(*offset, remove, insert);
        }

        for (offset, kind, data) in edits {
            match kind {
                Kind::Deletion => {
                    let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Removed {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    }));
                }
                Kind::Insertion => {
                    let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Inserted {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    }));
                }
            }
        }

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn hot_redo(state: &mut State, view: ViewId) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        if dse.read_only {
            debug_panic!();
            return;
        }

        let edits = dse.doc.data.hot_redo();
        dse.doc.modified = true;

        let len = dse.doc.data.len();
        for cursor in &mut vse.cursors.list {
            cursor.offset = cursor.offset.min(len);
            cursor.anchor = cursor.anchor.min(len);
        }

        for (offset, kind, data) in &edits {
            let (remove, insert) = match kind {
                Kind::Deletion => (data.len(), 0),
                Kind::Insertion => (0, data.len()),
            };

            dse.decs.edit(*offset, remove, insert);
            vse.decs.edit(*offset, remove, insert);
        }

        for (offset, kind, data) in edits {
            match kind {
                Kind::Deletion => {
                    let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Removed {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    }));
                }
                Kind::Insertion => {
                    let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Inserted {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    }));
                }
            }
        }

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorsChanged { view }));
    }

    pub fn saved(state: &mut State, doc: DocId) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        if !dse.doc.modified {
            return;
        }

        dse.doc.modified = false;

        let path = dse.doc.path.clone();
        if let Some(path) = path {
            let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Written {
                id: doc,
                path,
                bytes_written: dse.doc.data.len(),
            }));
        }
    }

    pub fn insert(state: &mut State, view: ViewId, text: String) {
        Self::execute_transaction(state, view, |_, cursors, _| {
            cursors
                .iter()
                .map(|cursor| Edit { offset: cursor.offset, remove: 0, insert: text.clone() })
                .collect()
        });
    }

    pub fn insert_at(state: &mut State, view: ViewId, text: String, offset: usize) {
        Self::execute_transaction(state, view, |_, _, _| {
            vec![Edit { offset, remove: 0, insert: text }]
        });
    }

    pub fn backspace(state: &mut State, view: ViewId) { Self::execute_remove(state, view, true); }

    pub fn delete(state: &mut State, view: ViewId) { Self::execute_remove(state, view, false); }

    pub fn remove(state: &mut State, view: ViewId, offset: usize, len: usize) {
        Self::execute_transaction(state, view, |_, _, _| {
            vec![Edit { offset, remove: len, insert: String::new() }]
        });
    }

    pub fn set_doc_path(state: &mut State, doc: DocId, path: Option<PathBuf>) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.doc.path = path.clone();

        let _ =
            state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::PathChanged { id: doc, path }));
    }

    pub fn set_doc_mode(state: &mut State, doc: DocId, mode: DocStoreTypes::Mode) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.mode = mode;

        let _ =
            state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::ModeChanged { id: doc, mode }));
    }

    pub fn push_view_mode(state: &mut State, view: ViewId, mode: ViewStoreTypes::Mode) {
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        if vse.modes.last() != Some(&mode) {
            vse.modes.push(mode);
        }

        let _ = state
            .fevent_tx
            .send(FEvent::View(ViewStoreTypes::Event::ModeChanged { id: view, mode }));
    }

    pub fn pop_view_mode(state: &mut State, view: ViewId) {
        let Some(vse) = state.view_store.get_mut(&view) else {
            return;
        };

        if vse.modes.len() > 1 {
            vse.modes.pop();
        }

        let mode = vse.mode();
        let _ = state
            .fevent_tx
            .send(FEvent::View(ViewStoreTypes::Event::ModeChanged { id: view, mode }));
    }

    pub fn set_doc_decorator(
        state: &mut State, doc: DocId, id: DocStoreTypes::DecorationId, dec: Box<dyn Decorator>,
    ) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.decs.decorators.insert(id, dec);
    }

    pub fn set_view_decorator(
        state: &mut State, view: ViewId, id: ViewStoreTypes::DecorationId, dec: Box<dyn Decorator>,
    ) {
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.decs.decorators.insert(id, dec);
    }

    pub fn remove_doc_decorator(state: &mut State, doc: DocId, id: DocStoreTypes::DecorationId) {
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.decs.decorators.remove(&id);
    }

    pub fn remove_view_decorator(
        state: &mut State, view: ViewId, id: ViewStoreTypes::DecorationId,
    ) {
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.decs.decorators.remove(&id);
    }

    pub fn set_active_window(state: &mut State, window: Option<WindowId>) {
        state.workspace.active_window = window;
    }

    pub fn can_quit(state: &State, _: &PState) -> Result<(), String> {
        for dse in state.doc_store.values() {
            if !dse.doc.modified {
                continue;
            }

            let name = dse
                .doc
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "SCRATCHPAD".to_string());

            return Err(format!("'{name}' has unsaved changes"));
        }

        Ok(())
    }

    fn execute_remove(state: &mut State, view: ViewId, bksp: bool) {
        Self::execute_transaction(state, view, |doc_data, cursors, _| {
            let mut edits = Vec::new();
            for cursor in cursors {
                if bksp {
                    if cursor.offset == 0 {
                        continue;
                    }

                    let lines = doc_data.lines();
                    let mut current_y = lines.saturating_sub(1);
                    for y in 0..lines {
                        if cursor.offset <= doc_data.get_line_end_byte(y) {
                            current_y = y;

                            break;
                        }
                    }

                    let start = doc_data.get_line_start_byte(current_y);
                    let remove = if cursor.offset == start {
                        1
                    } else {
                        let text = doc_data.slice(start..cursor.offset);
                        text.graphemes(true).last().map(|g| g.len()).unwrap_or(1)
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

                    let lines = doc_data.lines();
                    let mut current_y = lines.saturating_sub(1);
                    for y in 0..lines {
                        if cursor.offset < doc_data.get_line_end_byte(y) || y == lines - 1 {
                            current_y = y;
                            break;
                        }
                    }

                    let end = doc_data.get_line_end_byte(current_y);
                    let remove = if cursor.offset == end {
                        1
                    } else {
                        let text = doc_data.slice(cursor.offset..end);
                        text.graphemes(true).next().map(|g| g.len()).unwrap_or(1)
                    };

                    edits.push(Edit { offset: cursor.offset, remove, insert: String::new() });
                }
            }

            edits
        });
    }

    fn execute_transaction(
        state: &mut State, view: ViewId,
        edits: impl FnOnce(&mut PieceTable, &Vec<Cursor>, ViewStoreTypes::TabWidth) -> Vec<Edit>,
    ) {
        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        if dse.read_only {
            debug_panic!();
            return;
        }

        let mut cursors = vse.cursors.clone();
        let tab_width = vse.layout.tab_width;

        let mut edits = edits(&mut dse.doc.data, &cursors.list, tab_width);
        edits.sort_by_key(|e| std::cmp::Reverse(e.offset));
        edits.dedup_by_key(|e| e.offset);

        for edit in &edits {
            if edit.remove > 0 {
                let str = dse.doc.data.slice(edit.offset..edit.offset + edit.remove);

                dse.doc.data.remove(edit.offset, edit.remove);
                dse.doc.modified = true;

                let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Removed {
                    id: doc,
                    pos: edit.offset,
                    n: edit.remove,
                    str,
                }));
            }

            if !edit.insert.is_empty() {
                dse.doc.data.insert(edit.offset, &edit.insert);
                dse.doc.modified = true;

                let _ = state.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Inserted {
                    id: doc,
                    pos: edit.offset,
                    n: edit.insert.len(),
                    str: edit.insert.clone(),
                }));
            }

            dse.decs.edit(edit.offset, edit.remove, edit.insert.len());
            vse.decs.edit(edit.offset, edit.remove, edit.insert.len());
        }

        for cursor in &mut cursors.list {
            for edit in &edits {
                if edit.offset < cursor.offset {
                    // Before cursor.
                    if cursor.offset < edit.offset + edit.remove {
                        cursor.offset = edit.offset;
                    } else {
                        cursor.offset = cursor.offset + edit.insert.len() - edit.remove;
                    }
                } else if edit.offset == cursor.offset {
                    // On cursor.
                    cursor.offset += edit.insert.len();
                }

                if edit.offset < cursor.anchor {
                    if cursor.anchor < edit.offset + edit.remove {
                        cursor.anchor = edit.offset;
                    } else {
                        cursor.anchor = cursor.anchor + edit.insert.len() - edit.remove;
                    }
                } else if edit.offset == cursor.anchor {
                    // On cursor.
                    cursor.anchor += edit.insert.len();
                }
            }
        }

        cursors.normalize();

        debug_assert!(
            cursors
                .list
                .iter()
                .all(|c| c.offset <= dse.doc.data.len() && c.anchor <= dse.doc.data.len())
        );

        if cursors.list.is_empty() {
            return;
        }

        vse.cursors = cursors.clone();

        let _ = state.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::CursorMoved { view }));
    }
}
