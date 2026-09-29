use piece_table::{Kind, PieceTable, Slice};
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    protocols::action::ActionCommand,
    state::{DocId, DocStoreTypes, State, StateLock, ViewId, ViewStoreTypes},
    types::{Cursor, Motion, Pos},
    util,
};

struct Edit {
    offset: usize,
    remove: usize,
    insert: String,
}

pub struct ActionProtocol {
    state_lock: StateLock,

    rx: UnboundedReceiver<ActionCommand>,
}

impl ActionProtocol {
    pub fn new(state_lock: StateLock, rx: UnboundedReceiver<ActionCommand>) -> Self {
        Self { state_lock, rx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ActionCommand::MoveCursors { view, motion: direction, move_anchor } => {
                    self.move_cursors(view, direction, move_anchor);
                }
                ActionCommand::MoveCursorToPos { view, pos, move_anchor } => {
                    self.move_cursor_to_pos(view, pos, move_anchor);
                }
                ActionCommand::CreateCursorAtPos { view, pos } => {
                    self.create_cursor_at_pos(view, pos)
                }
                ActionCommand::CreateCursorsAtOffset { view, offsets } => {
                    self.create_cursors_at_offset(view, offsets)
                }
                ActionCommand::RemoveCursors { view, offsets } => {
                    self.remove_cursors(view, offsets)
                }

                ActionCommand::StartCommit { doc } => self.start_commit(doc),
                ActionCommand::EndCommit { doc } => self.end_commit(doc),

                ActionCommand::Undo { view } => self.undo(view),
                ActionCommand::HotRedo { view } => self.hot_redo(view),

                ActionCommand::Saved { doc } => self.saved(doc),

                ActionCommand::Insert { view, text } => self.insert(view, text),
                ActionCommand::InsertAt { view, text, offset } => {
                    self.insert_at(view, text, offset)
                }
                ActionCommand::Backspace { view } => self.backspace(view),
                ActionCommand::Delete { view } => self.delete(view),
                ActionCommand::Remove { view, offset, len } => self.remove(view, offset, len),

                ActionCommand::SetDocMode { doc, mode } => self.set_doc_mode(doc, mode),
                ActionCommand::SetViewMode { view, mode } => self.set_view_mode(view, mode),

                ActionCommand::CanQuit { tx } => self.can_quit(tx),
            }
        }
    }

    fn move_cursors(&self, view: ViewId, motion: Motion, move_anchor: bool) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };

        let mut cursors = vse.cursors.clone();
        for cursor in &mut cursors.list {
            util::apply_motion(cursor, motion, &vse, &dse);

            if move_anchor {
                cursor.anchor = cursor.offset;
            }
        }

        cursors.normalize();
        if cursors.list.is_empty() {
            return;
        }

        vse.cursors = cursors.clone();

        let view_event_tx = state.view_event_tx.clone();
        drop(guard);

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorMoved { view });
    }

    fn move_cursor_to_pos(&self, view: ViewId, pos: Pos, move_anchor: bool) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

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

        let view_event_tx = state.view_event_tx.clone();
        drop(guard);

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorMoved { view });
    }

    fn create_cursor_at_pos(&self, view: ViewId, pos: Pos) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };

        let offset = util::pos_to_offset(pos, vse, dse);

        vse.cursors.list.push(Cursor::new(offset, pos.x));
        vse.cursors.normalize();

        let view_event_tx = state.view_event_tx.clone();
        drop(guard);

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorsChanged { view });
    }

    fn create_cursors_at_offset(&self, view: ViewId, offsets: Vec<usize>) {
        let mut state = self.state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        for offset in offsets {
            vse.cursors.list.push(Cursor::new(offset, 0));
        }

        vse.cursors.normalize();

        let view_event_tx = state.view_event_tx.clone();
        drop(state);

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorMoved { view });
    }

    fn remove_cursors(&self, view: ViewId, offsets: Vec<usize>) {
        let mut state = self.state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.cursors.list.retain(|c| !offsets.contains(&c.offset));

        let view_event_tx = state.view_event_tx.clone();
        drop(state);

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorsChanged { view });
    }

    fn start_commit(&self, doc: DocId) {
        let mut state = self.state_lock.write();
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.doc.data.start_commit();
        drop(state);
    }

    fn end_commit(&self, doc: DocId) {
        let mut state = self.state_lock.write();
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.doc.data.end_commit();
        drop(state);
    }

    fn undo(&self, view: ViewId) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

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

        let doc_event_tx = state.doc_event_tx.clone();
        let view_event_tx = state.view_event_tx.clone();
        drop(guard);

        for (offset, kind, data) in edits {
            match kind {
                Kind::Deletion => {
                    let _ = doc_event_tx.send(DocStoreTypes::Event::Removed {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    });
                }
                Kind::Insertion => {
                    let _ = doc_event_tx.send(DocStoreTypes::Event::Inserted {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    });
                }
            }
        }

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorsChanged { view });
    }

    fn hot_redo(&self, view: ViewId) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

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

        let doc_event_tx = state.doc_event_tx.clone();
        let view_event_tx = state.view_event_tx.clone();
        drop(guard);

        for (offset, kind, data) in edits {
            match kind {
                Kind::Deletion => {
                    let _ = doc_event_tx.send(DocStoreTypes::Event::Removed {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    });
                }
                Kind::Insertion => {
                    let _ = doc_event_tx.send(DocStoreTypes::Event::Inserted {
                        id: doc,
                        pos: offset,
                        n: data.len(),
                        str: data,
                    });
                }
            }
        }

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorsChanged { view });
    }

    fn saved(&self, doc: DocId) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        if !dse.doc.modified {
            return;
        }

        dse.doc.modified = false;

        let tx = state.doc_event_tx.clone();
        let path = dse.doc.path.clone();
        let bytes_written = dse.doc.data.len();
        drop(guard);

        if let Some(path) = path {
            let _ = tx.send(DocStoreTypes::Event::Written { id: doc, path, bytes_written });
        }
    }

    fn insert(&self, view: ViewId, text: String) {
        self.execute_transaction(view, |_, cursors, _| {
            cursors
                .iter()
                .map(|cursor| Edit { offset: cursor.offset, remove: 0, insert: text.clone() })
                .collect()
        });
    }

    fn insert_at(&self, view: ViewId, text: String, offset: usize) {
        self.execute_transaction(view, |_, _, _| vec![Edit { offset, remove: 0, insert: text }]);
    }

    fn backspace(&self, view: ViewId) { self.execute_remove(view, true); }

    fn delete(&self, view: ViewId) { self.execute_remove(view, false); }

    fn remove(&self, view: ViewId, offset: usize, len: usize) {
        self.execute_transaction(view, |_, _, _| {
            vec![Edit { offset, remove: len, insert: String::new() }]
        });
    }

    fn set_doc_mode(&mut self, doc: DocId, mode: DocStoreTypes::Mode) {
        let mut state = self.state_lock.write();
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.mode = mode;

        let doc_event_tx = state.doc_event_tx.clone();
        drop(state);

        let _ = doc_event_tx.send(DocStoreTypes::Event::ModeChanged { id: doc, mode });
    }

    fn set_view_mode(&self, view: ViewId, mode: ViewStoreTypes::Mode) {
        let mut state = self.state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.mode = mode;

        let view_event_tx = state.view_event_tx.clone();
        drop(state);

        // Mode changes may include mode-line changes.
        let _ = view_event_tx.send(ViewStoreTypes::Event::ModeChanged { id: view, mode });
    }

    fn can_quit(&self, tx: oneshot::Sender<Result<(), String>>) {
        let state = self.state_lock.read();
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

            let _ = tx.send(Err(format!("'{name}' has unsaved changes")));

            return;
        }
        drop(state);

        let _ = tx.send(Ok(()));
    }

    fn execute_remove(&self, view: ViewId, bksp: bool) {
        self.execute_transaction(view, |doc_data, cursors, _| {
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
        &self, view: ViewId,
        edits: impl FnOnce(&mut PieceTable, &Vec<Cursor>, ViewStoreTypes::TabWidth) -> Vec<Edit>,
    ) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut state.view_store, &mut state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(doc_id) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };

        let mut cursors = vse.cursors.clone();
        let tab_width = vse.tab_width;

        let mut edits = edits(&mut dse.doc.data, &cursors.list, tab_width);
        edits.sort_by_key(|e| std::cmp::Reverse(e.offset));
        edits.dedup_by_key(|e| e.offset);

        for edit in &edits {
            if edit.remove > 0 {
                let str = dse.doc.data.slice(edit.offset..edit.offset + edit.remove);

                dse.doc.data.remove(edit.offset, edit.remove);
                dse.doc.modified = true;

                let _ = state.doc_event_tx.send(DocStoreTypes::Event::Removed {
                    id: doc_id,
                    pos: edit.offset,
                    n: edit.remove,
                    str,
                });
            }

            if !edit.insert.is_empty() {
                dse.doc.data.insert(edit.offset, &edit.insert);
                dse.doc.modified = true;

                let _ = state.doc_event_tx.send(DocStoreTypes::Event::Inserted {
                    id: doc_id,
                    pos: edit.offset,
                    n: edit.insert.len(),
                    str: edit.insert.clone(),
                });
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

        let view_event_tx = state.view_event_tx.clone();
        drop(guard);

        let _ = view_event_tx.send(ViewStoreTypes::Event::CursorMoved { view });
    }
}
