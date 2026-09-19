use std::collections::HashMap;

use piece_table::{PieceTable, Slice};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    protocols::{screen::ScreenCommand, view::ViewCommand},
    render,
    state::{
        DocumentId,
        DocumentStoreTypes::Mode as DocumentMode,
        State, StateLock, ViewId,
        ViewStoreTypes::{Mode as ViewMode, TabWidth},
    },
    types::{Cursor, Direction, Pos},
};

struct Edit {
    offset: usize,
    remove: usize,
    insert: String,
}

pub enum ActionCommand {
    MoveCursors { view: ViewId, direction: Direction },
    MoveCursorToPos { view: ViewId, pos: Pos },
    CreateCursorAtPos { view: ViewId, pos: Pos },
    RemoveCursor { view: ViewId, offset: usize },

    StartCommit { doc: DocumentId },
    EndCommit { doc: DocumentId },

    Insert { view: ViewId, text: String },
    InsertAt { view: ViewId, text: String, offset: usize },
    Backspace { view: ViewId },
    Delete { view: ViewId },
    Remove { view: ViewId, offset: usize, len: usize },

    SetDocumentMode { doc: DocumentId, mode: DocumentMode },
    SetViewMode { view: ViewId, mode: ViewMode },
}

pub struct ActionProtocol {
    state_lock: StateLock,

    rx: UnboundedReceiver<ActionCommand>,
    view_tx: UnboundedSender<ViewCommand>,
    screen_tx: UnboundedSender<ScreenCommand>,
}

impl ActionProtocol {
    pub fn new(
        state_lock: StateLock, rx: UnboundedReceiver<ActionCommand>,
        view_tx: UnboundedSender<ViewCommand>, screen_tx: UnboundedSender<ScreenCommand>,
    ) -> Self {
        Self { state_lock, rx, view_tx, screen_tx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                ActionCommand::MoveCursors { view, direction } => {
                    self.move_cursors(view, direction)
                }
                ActionCommand::MoveCursorToPos { view, pos } => self.move_cursor_to_pos(view, pos),
                ActionCommand::CreateCursorAtPos { view, pos } => {
                    self.create_cursor_at_pos(view, pos)
                }
                ActionCommand::RemoveCursor { view, offset } => self.remove_cursor(view, offset),

                ActionCommand::StartCommit { doc } => self.start_commit(doc),
                ActionCommand::EndCommit { doc } => self.end_commit(doc),

                ActionCommand::Insert { view, text } => self.insert(view, text),
                ActionCommand::InsertAt { view, text, offset } => {
                    self.insert_at(view, text, offset)
                }
                ActionCommand::Backspace { view } => self.backspace(view),
                ActionCommand::Delete { view } => self.delete(view),
                ActionCommand::Remove { view, offset, len } => self.remove(view, offset, len),

                ActionCommand::SetDocumentMode { doc, mode } => self.set_doc_mode(doc, mode),
                ActionCommand::SetViewMode { view, mode } => self.set_view_mode(view, mode),
            }
        }
    }

    fn move_cursors(&self, view: ViewId, direction: Direction) {
        let mut state = self.state_lock.write();
        let Some((vse, dse)) = state.vse_and_dse_mut(view) else {
            debug_panic!();
            return;
        };

        let mut cursors = vse.cursors.clone();
        let tab_width = vse.tab_width;
        let lines = dse.doc.data.lines();

        for cursor in &mut cursors.list {
            let mut curr_y = lines.saturating_sub(1);
            for y in 0..lines {
                if cursor.offset >= dse.doc.data.get_line_start_byte(y)
                    && (cursor.offset < dse.doc.data.get_line_end_byte(y) || y == lines - 1)
                {
                    curr_y = y;

                    break;
                }
            }

            let start = dse.doc.data.get_line_start_byte(curr_y);
            let end = dse.doc.data.get_line_end_byte(curr_y);
            let line = dse.doc.data.slice(start..end);

            let mut decs = Vec::new();
            dse.decs.range(start, end, &mut decs);
            vse.decs.range(start, end, &mut decs);

            let (vom, _) = render::layout_vom(&line, start, tab_width, &decs);
            let idx = vom.iter().position(|vo| vo.offset == cursor.offset).unwrap_or(0);

            match direction {
                Direction::Up => {
                    if curr_y == 0 {
                        cursor.offset = 0;

                        continue;
                    }

                    let start = dse.doc.data.get_line_start_byte(curr_y - 1);
                    let end = dse.doc.data.get_line_end_byte(curr_y - 1);
                    let (vom, _) = render::layout_vom(
                        &dse.doc.data.slice(start..end),
                        start,
                        tab_width,
                        &decs,
                    );

                    let vo = vom
                        .iter()
                        .rev()
                        .find(|vo| vo.visual_x <= cursor.pref_x)
                        .unwrap_or_else(|| vom.first().unwrap());
                    cursor.offset = vo.offset;
                }
                Direction::Down => {
                    if curr_y + 1 == lines {
                        cursor.offset = dse.doc.data.len();

                        continue;
                    }

                    let start = dse.doc.data.get_line_start_byte(curr_y + 1);
                    let end = dse.doc.data.get_line_end_byte(curr_y + 1);
                    let (vom, _) = render::layout_vom(
                        &dse.doc.data.slice(start..end),
                        start,
                        tab_width,
                        &decs,
                    );

                    let vo = vom
                        .iter()
                        .rev()
                        .find(|vo| vo.visual_x <= cursor.pref_x)
                        .unwrap_or_else(|| vom.first().unwrap());
                    cursor.offset = vo.offset;
                }
                Direction::Left => {
                    if idx > 0 {
                        cursor.offset = vom[idx - 1].offset;
                        cursor.pref_x = vom[idx - 1].visual_x;
                    } else if curr_y > 0 {
                        let start = dse.doc.data.get_line_start_byte(curr_y - 1);
                        let end = dse.doc.data.get_line_end_byte(curr_y - 1);
                        let (vom, _) = render::layout_vom(
                            &dse.doc.data.slice(start..end),
                            start,
                            tab_width,
                            &decs,
                        );

                        if let Some(last) = vom.last() {
                            cursor.offset = last.offset;
                            cursor.pref_x = last.visual_x;
                        } else {
                            cursor.offset = start;
                            cursor.pref_x = 0;
                        }
                    }
                }
                Direction::Right => {
                    if idx + 1 < vom.len() {
                        cursor.offset = vom[idx + 1].offset;
                        cursor.pref_x = vom[idx + 1].visual_x;
                    } else if curr_y + 1 < lines {
                        cursor.offset = dse.doc.data.get_line_start_byte(curr_y + 1);
                        cursor.pref_x = 0;
                    }
                }
            }
        }

        cursors.list.sort_by_key(|c| c.offset);
        cursors.list.dedup_by_key(|c| c.offset);

        vse.cursors = cursors.clone();

        let Some(cursor) = cursors.list.first() else { return };
        let Some(window) = state.workspace.active_window else {
            // If no active window, nothing needs to scroll.
            return;
        };

        let pos = self
            .offset_to_pos(&state, view, cursor.offset)
            .expect("Beginning of function checks it");
        drop(state);

        let _ = self.view_tx.send(ViewCommand::ScrollIfNeeded { window, view, pos });
    }

    fn move_cursor_to_pos(&self, view: ViewId, pos: Pos) {
        let mut state = self.state_lock.write();
        let Some(offset) = self.pos_to_offset(&state, view, pos) else { return };
        let vse = state.view_store.get_mut(&view).expect("self.pos_to_offset checks it");

        vse.cursors.list.clear();
        vse.cursors.list.push(Cursor::new(offset, pos.x));

        let Some(window) = state.workspace.active_window else {
            // If no active window, nothing needs to scroll.
            return;
        };
        drop(state);

        let _ = self.view_tx.send(ViewCommand::ScrollIfNeeded { window, view, pos });
    }

    fn create_cursor_at_pos(&self, view: ViewId, pos: Pos) {
        let mut state = self.state_lock.write();

        let Some(offset) = self.pos_to_offset(&state, view, pos) else { return };
        let vse = state.view_store.get_mut(&view).expect("self.pos_to_offset checks it");

        vse.cursors.list.push(Cursor::new(offset, pos.x));
        vse.cursors.list.sort_by_key(|c| c.offset);
        vse.cursors.list.dedup_by_key(|c| c.offset);

        drop(state);

        // Explicitly redraw the screen after creating new cursors.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    fn remove_cursor(&mut self, view: ViewId, offset: usize) {
        let mut state = self.state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.cursors.list.retain(|c| c.offset != offset);
        drop(state);

        // Explicitly redraw the screen after removing cursors.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    fn start_commit(&mut self, doc: DocumentId) {
        let mut state = self.state_lock.write();
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.doc.data.start_commit();
        drop(state);
    }

    fn end_commit(&mut self, doc: DocumentId) {
        let mut state = self.state_lock.write();
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.doc.data.end_commit();
        drop(state);
    }

    fn insert(&self, view: ViewId, text: String) {
        if text == " " || text == "\n" || text == "\t" {
            let mut state = self.state_lock.write();
            let Some((_, dse)) = state.vse_and_dse_mut(view) else {
                debug_panic!();
                return;
            };

            dse.doc.data.end_commit();
            dse.doc.data.start_commit();
            drop(state);
        }

        if text != "\t" {
            self.execute_insert(view, |_| text.clone());
        } else {
            let state = self.state_lock.read();
            let Some(vse) = state.view_store.get(&view) else {
                debug_panic!();
                return;
            };

            let mut cursor_xs = HashMap::new();
            for offset in vse.cursors.list.iter().map(|c| c.offset) {
                if let Some(pos) = self.offset_to_pos(&state, view, offset) {
                    cursor_xs.insert(offset, pos.x);
                }
            }
            drop(state);

            self.execute_transaction(view, |_, cursors, tab_width| {
                cursors
                    .iter()
                    .map(|cursor| {
                        let x = cursor_xs.get(&cursor.offset).cloned().unwrap_or(0);

                        Edit {
                            offset: cursor.offset,
                            remove: 0,
                            insert: " ".repeat(*tab_width - (x % *tab_width)),
                        }
                    })
                    .collect()
            });
        }
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

    fn set_doc_mode(&mut self, doc: DocumentId, mode: DocumentMode) {
        let mut state = self.state_lock.write();
        let Some(dse) = state.doc_store.get_mut(&doc) else {
            debug_panic!();
            return;
        };

        dse.mode = mode;
        drop(state);

        // Mode changes may include mode-line changes.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    fn set_view_mode(&mut self, view: ViewId, mode: ViewMode) {
        let mut state = self.state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.mode = mode;
        drop(state);

        // Mode changes may include mode-line changes.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    fn execute_insert<F: Fn(&Cursor) -> String>(&self, view: ViewId, text: F) {
        self.execute_transaction(view, |_, cursors, _| {
            cursors
                .iter()
                .map(|cursor| Edit { offset: cursor.offset, remove: 0, insert: text(cursor) })
                .collect()
        });
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
                        let text = doc_data.slice(start..cursor.offset).to_string();
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
                        let text = doc_data.slice(cursor.offset..end).to_string();
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
        edits: impl FnOnce(&mut PieceTable, &Vec<Cursor>, TabWidth) -> Vec<Edit>,
    ) {
        let mut guard = self.state_lock.write();
        // Fix the borrow checker.
        let state = &mut *guard;

        let Some((vse, dse)) = state.vse_and_dse_mut(view) else {
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
                dse.doc.data.remove(edit.offset, edit.remove);
                dse.doc.modified = true;
            }

            if !edit.insert.is_empty() {
                dse.doc.data.insert(edit.offset, &edit.insert);
                dse.doc.modified = true;
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
            }
        }

        cursors.list.sort_by_key(|c| c.offset);
        cursors.list.dedup_by_key(|c| c.offset);

        vse.cursors = cursors.clone();

        let window = state.workspace.active_window;
        drop(guard);

        let _ = self.view_tx.send(ViewCommand::Update { view });

        let mut state = self.state_lock.write();
        let Some(cursor) = cursors.list.first() else { return };

        let pos = self
            .offset_to_pos(&state, view, cursor.offset)
            .expect("Beginning of function checks it");

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.cursors.list[0].pref_x = pos.x;
        drop(state);

        let Some(window) = window else {
            // If no active window, nothing needs to scroll.
            return;
        };

        let _ = self.view_tx.send(ViewCommand::ScrollIfNeeded { window, view, pos });
    }

    fn pos_to_offset(&self, state: &State, view: ViewId, pos: Pos) -> Option<usize> {
        let Some((vse, dse)) = state.vse_and_dse(view) else {
            debug_panic!();
            return None;
        };

        let lines = dse.doc.data.lines();
        let tab_width = vse.tab_width;

        // lines are one indexed.
        let y = pos.y.min(lines.saturating_sub(1));

        let start = dse.doc.data.get_line_start_byte(y);
        let end = dse.doc.data.get_line_end_byte(y);
        let line = dse.doc.data.slice(start..end);

        let mut decs = Vec::new();
        dse.decs.range(start, end, &mut decs);
        vse.decs.range(start, end, &mut decs);

        let (vom, _) = render::layout_vom(&line, start, tab_width, &decs);
        let offset =
            vom.iter().rev().find(|vo| vo.visual_x <= pos.x).map(|vo| vo.offset).unwrap_or(start);

        Some(offset)
    }

    fn offset_to_pos(&self, state: &State, view: ViewId, offset: usize) -> Option<Pos> {
        let Some((vse, dse)) = state.vse_and_dse(view) else {
            debug_panic!();
            return None;
        };

        let tab_width = vse.tab_width;
        let lines = dse.doc.data.lines();

        let mut target = lines.saturating_sub(1);
        for y in 0..lines {
            let start = dse.doc.data.get_line_start_byte(y);
            let end = dse.doc.data.get_line_end_byte(y);

            if offset >= start && (offset < end || y == lines - 1) {
                target = y;

                break;
            }
        }

        let start = dse.doc.data.get_line_start_byte(target);
        let end = dse.doc.data.get_line_end_byte(target);
        let line = dse.doc.data.slice(start..end);

        let mut decs = Vec::new();
        dse.decs.range(start, end, &mut decs);
        vse.decs.range(start, end, &mut decs);

        let (vom, _) = render::layout_vom(&line, start, tab_width, &decs);
        let x = vom
            .iter()
            .find(|vo| vo.offset >= offset)
            .map(|vo| vo.visual_x)
            .unwrap_or_else(|| vom.last().map(|vo| vo.visual_x).unwrap_or(0));

        Some(Pos::new(x, target))
    }
}
