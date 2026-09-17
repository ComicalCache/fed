use piece_table::{PieceTable, Slice};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    protocols::{screen::ScreenCommand, view::ViewCommand},
    render,
    state::{
        DocumentId,
        DocumentStoreTypes::Mode as DocumentMode,
        StateLock, ViewId,
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
    MoveCursorTo { view: ViewId, pos: Pos },
    CreateCursor { view: ViewId, pos: Pos },
    RemoveCursor { view: ViewId, pos: Pos },

    StartCommit { doc: DocumentId },
    EndCommit { doc: DocumentId },

    Insert { view: ViewId, text: String },
    InsertAt { view: ViewId, text: String, offset: usize },
    InsertNewline { view: ViewId },
    InsertTab { view: ViewId },
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
                    self.move_cursors(view, direction).await
                }
                ActionCommand::MoveCursorTo { view, pos } => self.move_cursor_to(view, pos).await,
                ActionCommand::CreateCursor { view, pos } => self.create_cursor(view, pos).await,
                ActionCommand::RemoveCursor { view, pos } => self.remove_cursor(view, pos).await,

                ActionCommand::StartCommit { doc } => self.start_commit(doc).await,
                ActionCommand::EndCommit { doc } => self.end_commit(doc).await,

                ActionCommand::Insert { view, text } => self.insert(view, text).await,
                ActionCommand::InsertAt { view, text, offset } => {
                    self.insert_at(view, text, offset).await
                }
                ActionCommand::InsertNewline { view } => self.insert_newline(view).await,
                ActionCommand::InsertTab { view } => self.insert_tab(view).await,
                ActionCommand::Backspace { view } => self.backspace(view).await,
                ActionCommand::Delete { view } => self.delete(view).await,
                ActionCommand::Remove { view, offset, len } => self.remove(view, offset, len).await,

                ActionCommand::SetDocumentMode { doc, mode } => {
                    self.set_document_mode(doc, mode).await
                }
                ActionCommand::SetViewMode { view, mode } => self.set_view_mode(view, mode).await,
            }
        }
    }

    async fn move_cursors(&self, view: ViewId, direction: Direction) {
        let state = self.state_lock.read();

        let Some(vse) = state.view_store.get(&view) else { return };
        let doc = vse.doc;
        let mut cursors = vse.cursors.clone();
        let tab_width = vse.tab_width;

        drop(state);

        for cursor in &mut cursors.list {
            match direction {
                Direction::Up => {
                    cursor.pos.y = cursor.pos.y.saturating_sub(1);

                    let stops = self.visual_cursor_stops(view, cursor.pos.y, tab_width).await;
                    cursor.pos.x = stops
                        .iter()
                        .rev()
                        .find(|&&x| x <= cursor.pref_x)
                        .cloned()
                        .unwrap_or_else(|| stops.first().cloned().unwrap_or(0));
                }
                Direction::Down => {
                    let state = self.state_lock.read();

                    let Some(dse) = state.document_store.get(&doc) else { return };
                    let lines = dse.doc.data.lines();

                    drop(state);

                    if cursor.pos.y < lines.saturating_sub(1) {
                        cursor.pos.y += 1;
                    }

                    let stops = self.visual_cursor_stops(view, cursor.pos.y, tab_width).await;
                    cursor.pos.x = stops
                        .iter()
                        .rev()
                        .find(|&&x| x <= cursor.pref_x)
                        .cloned()
                        .unwrap_or_else(|| stops.first().cloned().unwrap_or(0));
                }
                Direction::Left => {
                    let stops = self.visual_cursor_stops(view, cursor.pos.y, tab_width).await;
                    cursor.pos.x = stops
                        .iter()
                        .rev()
                        .find(|&&x| x < cursor.pos.x)
                        .cloned()
                        .unwrap_or_else(|| stops.first().cloned().unwrap_or(0));
                    cursor.pref_x = cursor.pos.x;
                }
                Direction::Right => {
                    let stops = self.visual_cursor_stops(view, cursor.pos.y, tab_width).await;

                    if let Some(x) = stops.into_iter().find(|&x| x > cursor.pos.x) {
                        cursor.pos.x = x;
                        cursor.pref_x = cursor.pos.x;
                    }
                }
            }
        }

        cursors.list.sort_by(|a, b| a.pos.y.cmp(&b.pos.y).then(a.pos.x.cmp(&b.pos.x)));
        cursors.list.dedup_by_key(|c| c.pos);

        let mut state = self.state_lock.write();

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.cursors = cursors.clone();

        drop(state);

        if let Some(cursor) = cursors.list.first() {
            let _ = self.view_tx.send(ViewCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }

    async fn move_cursor_to(&self, view: ViewId, pos: Pos) {
        let state = self.state_lock.read();

        let Some((vse, dse)) = state.view_and_doc(view) else { return };
        let mut cursors = vse.cursors.clone();
        let tab_width = vse.tab_width;
        let lines = dse.doc.data.lines();

        // lines are one indexed.
        let y = pos.y.min(lines - 1);
        let stops = self.visual_cursor_stops(view, y, tab_width).await;

        drop(state);

        let x = stops
            .iter()
            .rev()
            .find(|&&x| x <= pos.x)
            .cloned()
            .unwrap_or_else(|| stops.first().cloned().unwrap_or(0));

        // Moving to a specific location collapses all cursors.
        cursors.list.drain(1..);
        cursors.list[0] = Cursor::new(Pos::new(x, y), x);

        let mut state = self.state_lock.write();

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.cursors = cursors.clone();

        drop(state);

        if let Some(cursor) = cursors.list.first() {
            let _ = self.view_tx.send(ViewCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }

    async fn create_cursor(&self, view: ViewId, pos: Pos) {
        let state = self.state_lock.read();

        let Some((vse, dse)) = state.view_and_doc(view) else { return };
        let mut cursors = vse.cursors.clone();
        let tab_width = vse.tab_width;
        let lines = dse.doc.data.lines();

        // lines are one indexed.
        let y = pos.y.min(lines.saturating_sub(1));
        let stops = self.visual_cursor_stops(view, y, tab_width).await;

        drop(state);

        let x = stops
            .iter()
            .rev()
            .find(|&&x| x <= pos.x)
            .cloned()
            .unwrap_or_else(|| stops.first().cloned().unwrap_or(0));

        cursors.list.push(Cursor::new(Pos::new(x, y), x));
        cursors.list.sort_by(|a, b| a.pos.y.cmp(&b.pos.y).then(a.pos.x.cmp(&b.pos.x)));
        cursors.list.dedup_by_key(|c| c.pos);

        let mut state = self.state_lock.write();

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.cursors = cursors;

        drop(state);

        // Explicitly redraw the screen after creating new cursors.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    async fn remove_cursor(&mut self, view: ViewId, pos: Pos) {
        let mut state = self.state_lock.write();

        let Some(vse) = state.view_store.get_mut(&view) else { return };
        vse.cursors.list.retain(|c| c.pos != pos);

        drop(state);

        // Explicitly redraw the screen after removing cursors.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    async fn start_commit(&mut self, doc: DocumentId) {
        let mut state = self.state_lock.write();

        let Some(dse) = state.document_store.get_mut(&doc) else { return };

        dse.doc.data.start_commit();

        drop(state);
    }

    async fn end_commit(&mut self, doc: DocumentId) {
        let mut state = self.state_lock.write();

        let Some(dse) = state.document_store.get_mut(&doc) else { return };

        dse.doc.data.end_commit();

        drop(state);
    }

    async fn insert(&self, view: ViewId, text: String) {
        if text == " " {
            let mut state = self.state_lock.write();

            let Some((_, dse)) = state.view_and_doc_mut(view) else { return };

            dse.doc.data.end_commit();
            dse.doc.data.start_commit();

            drop(state);
        }

        self.execute_insert(view, |_, _| text.clone()).await;
    }

    async fn insert_at(&self, view: ViewId, text: String, offset: usize) {
        self.execute_transaction(view, |_, _, _| vec![Edit { offset, remove: 0, insert: text }])
            .await;
    }

    async fn insert_newline(&self, view: ViewId) {
        let mut state = self.state_lock.write();

        let Some((_, dse)) = state.view_and_doc_mut(view) else { return };

        dse.doc.data.end_commit();
        dse.doc.data.start_commit();

        drop(state);

        self.execute_insert(view, |_, _| "\n".to_string()).await;
    }

    async fn insert_tab(&self, view: ViewId) {
        let mut state = self.state_lock.write();

        let Some((_, dse)) = state.view_and_doc_mut(view) else { return };

        dse.doc.data.end_commit();
        dse.doc.data.start_commit();

        drop(state);

        self.execute_insert(view, |cursor, tab_width| {
            " ".repeat(*tab_width - (cursor.pos.x % *tab_width))
        })
        .await;
    }

    async fn backspace(&self, view: ViewId) { self.execute_remove(view, true).await; }

    async fn delete(&self, view: ViewId) { self.execute_remove(view, false).await; }

    async fn remove(&self, view: ViewId, offset: usize, len: usize) {
        self.execute_transaction(view, |_, _, _| {
            vec![Edit { offset, remove: len, insert: String::new() }]
        })
        .await;
    }

    async fn set_document_mode(&mut self, doc: DocumentId, mode: DocumentMode) {
        let mut state = self.state_lock.write();

        let Some(dse) = state.document_store.get_mut(&doc) else { return };

        dse.mode = mode;

        drop(state);

        // Mode changes may include mode-line changes.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    async fn set_view_mode(&mut self, view: ViewId, mode: ViewMode) {
        let mut state = self.state_lock.write();

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.mode = mode;

        drop(state);

        // Mode changes may include mode-line changes.
        let _ = self.screen_tx.send(ScreenCommand::Render);
    }

    async fn visual_cursor_stops(&self, view: ViewId, y: usize, tab_width: TabWidth) -> Vec<usize> {
        let state = self.state_lock.read();

        let Some((vse, dse)) = state.view_and_doc(view) else { return vec![0] };
        let start = dse.doc.data.get_line_start_byte(y);
        let end = dse.doc.data.get_line_end_byte(y);
        let line = dse.doc.data.slice(start..end);

        let mut decs = Vec::new();
        dse.decs.range(start, end, &mut decs);
        vse.decs.range(start, end, &mut decs);

        drop(state);

        let (layout, _) = render::layout(&line, start, tab_width, &decs);

        if layout.visual_cursor_stops.is_empty() { vec![0] } else { layout.visual_cursor_stops }
    }

    async fn execute_insert<F: Fn(&Cursor, TabWidth) -> String>(&self, view: ViewId, text: F) {
        self.execute_transaction(view, |_, cursors, tab_width| {
            cursors
                .iter()
                .map(|(cursor, offset)| Edit {
                    offset: *offset,
                    remove: 0,
                    insert: text(cursor, tab_width),
                })
                .collect()
        })
        .await;
    }

    async fn execute_remove(&self, view: ViewId, bksp: bool) {
        self.execute_transaction(view, |doc_data, cursors, _| {
            let mut edits = Vec::new();

            for (cursor, offset) in cursors {
                if bksp {
                    if *offset == 0 {
                        continue;
                    }

                    let start = doc_data.get_line_start_byte(cursor.pos.y);
                    let remove = if *offset == start {
                        1
                    } else {
                        let text = doc_data.slice(start..*offset).to_string();
                        text.graphemes(true).last().map(|g| g.len()).unwrap_or(1)
                    };

                    edits.push(Edit { offset: *offset - remove, remove, insert: String::new() });
                } else {
                    if *offset >= doc_data.len() {
                        continue;
                    }

                    let end = doc_data.get_line_end_byte(cursor.pos.y);
                    let remove = if *offset == end {
                        1
                    } else {
                        let text = doc_data.slice(*offset..end).to_string();
                        text.graphemes(true).next().map(|g| g.len()).unwrap_or(1)
                    };

                    edits.push(Edit { offset: *offset, remove, insert: String::new() });
                }
            }

            edits
        })
        .await;
    }

    async fn execute_transaction(
        &self, view: ViewId,
        edits: impl FnOnce(&mut PieceTable, &Vec<(Cursor, usize)>, TabWidth) -> Vec<Edit>,
    ) {
        let mut state = self.state_lock.write();

        let Some((vse, dse)) = state.view_and_doc_mut(view) else { return };
        let mut cursors = vse.cursors.clone();
        let tab_width = vse.tab_width;

        let mut decs = Vec::new();

        // Map 2D cursors to 1D.
        let mut cursors_1d: Vec<_> = cursors
            .list
            .iter()
            .cloned()
            .map(|c| {
                let start = dse.doc.data.get_line_start_byte(c.pos.y);
                let end = dse.doc.data.get_line_end_byte(c.pos.y);
                let line = dse.doc.data.slice(start..end).to_string();

                decs.clear();
                dse.decs.range(start, end, &mut decs);
                vse.decs.range(start, end, &mut decs);

                let (layout, _) = render::layout(&line, start, tab_width, &decs);

                let offset = layout
                    .visual_offset_mapping
                    .iter()
                    .rev()
                    .find(|vo| vo.visual_x <= c.pos.x)
                    .map(|vo| vo.offset)
                    .unwrap_or(start);

                (c, offset)
            })
            .collect();

        let mut edits = edits(&mut dse.doc.data, &cursors_1d, tab_width);
        edits.sort_by_key(|e| std::cmp::Reverse(e.offset));
        edits.dedup_by_key(|e| e.offset);

        // Apply `Edits`.
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

        // Shift cursors in 1D space.
        for (cursor, offset) in &mut cursors_1d {
            for edit in &edits {
                if edit.offset < *offset {
                    // Before cursor.
                    if *offset < edit.offset + edit.remove {
                        *offset = edit.offset;
                    } else {
                        *offset = *offset + edit.insert.len() - edit.remove;
                    }
                } else if edit.offset == *offset {
                    // On cursor.
                    *offset += edit.insert.len();
                }
            }

            // Map 1D cursors to 2D.
            let lines = dse.doc.data.lines();
            let mut target_line = lines.saturating_sub(1);
            for y in 0..lines {
                let start = dse.doc.data.get_line_start_byte(y);
                let end = dse.doc.data.get_line_end_byte(y);

                if *offset >= start && (*offset < end || y == lines - 1) {
                    target_line = y;
                    break;
                }
            }

            let start = dse.doc.data.get_line_start_byte(target_line);
            let end = dse.doc.data.get_line_end_byte(target_line);
            let line = dse.doc.data.slice(start..end).to_string();

            decs.clear();
            dse.decs.range(start, end, &mut decs);
            vse.decs.range(start, end, &mut decs);

            let (layout, _) = render::layout(&line, start, tab_width, &decs);

            cursor.pos.y = target_line;
            cursor.pos.x = layout
                .visual_offset_mapping
                .iter()
                .find(|vo| vo.offset >= *offset)
                .map(|vo| vo.visual_x)
                .unwrap_or_else(|| layout.visual_cursor_stops.last().cloned().unwrap_or(0));
            cursor.pref_x = cursor.pos.x;
        }

        cursors.list = cursors_1d.into_iter().map(|(c, _)| c).collect();

        cursors.list.sort_by(|a, b| a.pos.y.cmp(&b.pos.y).then(a.pos.x.cmp(&b.pos.x)));
        cursors.list.dedup_by_key(|c| c.pos);

        let Some(vse) = state.view_store.get_mut(&view) else { return };

        vse.cursors = cursors.clone();

        drop(state);

        let _ = self.view_tx.send(ViewCommand::Update { view });
        if let Some(cursor) = cursors.list.first() {
            let _ = self.view_tx.send(ViewCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }
}
