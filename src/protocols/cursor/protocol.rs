use fed_core::{CoreCommandSender, DocumentId};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    protocols::buffer::BufferCommand,
    render,
    state::{
        DocumentStoreTypes::{self, Decorations as DocDecorations},
        State, ViewId,
        ViewStoreTypes::{self, Decorations as ViewDecorations},
    },
    types::{Cursor, Direction, Pos},
};

pub enum CursorCommand {
    Move { view: ViewId, direction: Direction },
    MoveTo { view: ViewId, pos: Pos },
}

pub struct CursorProtocol {
    state: State,

    rx: UnboundedReceiver<CursorCommand>,
    buffer_tx: UnboundedSender<BufferCommand>,
    core_tx: CoreCommandSender,
}

impl CursorProtocol {
    pub fn new(
        state: State, rx: UnboundedReceiver<CursorCommand>,
        buffer_tx: UnboundedSender<BufferCommand>, core_tx: CoreCommandSender,
    ) -> Self {
        Self { state, rx, buffer_tx, core_tx }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                CursorCommand::Move { view, direction } => self.r#move(view, direction).await,
                CursorCommand::MoveTo { view, pos } => self.move_to(view, pos).await,
            }
        }
    }

    async fn r#move(&self, view: ViewId, direction: Direction) {
        let (doc, mut cursors, tab_width, doc_decs, view_decs) = {
            let view_store = self.state.view_store.read().unwrap();
            let doc_store = self.state.document_store.read().unwrap();

            let Some(view) = view_store.get(&view) else {
                return;
            };

            let Some(&doc_id) = view.get::<DocumentId>() else {
                return;
            };
            let Some(doc) = doc_store.get(&doc_id) else {
                return;
            };

            let cursors = view.get::<ViewStoreTypes::Cursors>().cloned().unwrap_or_default();
            let tab_width =
                view.get::<ViewStoreTypes::TabWidth>().map(|&tab_width| *tab_width).unwrap_or(4);

            let doc_decs = doc.get::<DocumentStoreTypes::Decorations>().cloned();
            let view_decs = view.get::<ViewStoreTypes::Decorations>().cloned();

            (doc_id, cursors, tab_width, doc_decs, view_decs)
        };
        let (doc_decs, view_decs) = (doc_decs.as_ref(), view_decs.as_ref());

        for cursor in &mut cursors.list {
            match direction {
                Direction::Up => {
                    cursor.pos.y = cursor.pos.y.saturating_sub(1);

                    let stops =
                        self.cursor_stops(doc, cursor.pos.y, tab_width, doc_decs, view_decs).await;
                    cursor.pos.x =
                        stops.into_iter().rev().find(|&x| x <= cursor.pref_x).unwrap_or(0);
                }
                Direction::Down => {
                    if let Ok(lines) = self.core_tx.lines(doc).await {
                        if cursor.pos.y < lines.saturating_sub(1) {
                            cursor.pos.y += 1;
                        }
                    }

                    let stops =
                        self.cursor_stops(doc, cursor.pos.y, tab_width, doc_decs, view_decs).await;
                    cursor.pos.x =
                        stops.into_iter().rev().find(|&x| x <= cursor.pref_x).unwrap_or(0);
                }
                Direction::Left => {
                    let stops =
                        self.cursor_stops(doc, cursor.pos.y, tab_width, doc_decs, view_decs).await;
                    cursor.pos.x = stops.into_iter().rev().find(|&x| x < cursor.pos.x).unwrap_or(0);
                    cursor.pref_x = cursor.pos.x;
                }
                Direction::Right => {
                    let stops =
                        self.cursor_stops(doc, cursor.pos.y, tab_width, doc_decs, view_decs).await;

                    if let Some(x) = stops.into_iter().find(|&x| x > cursor.pos.x) {
                        cursor.pos.x = x;
                        cursor.pref_x = cursor.pos.x;
                    }
                }
            }
        }

        let mut view_store = self.state.view_store.write().unwrap();
        if let Some(view) = view_store.get_mut(&view) {
            view.insert(cursors.clone());
        }
        drop(view_store);

        if let Some(cursor) = cursors.list.first() {
            let _ = self.buffer_tx.send(BufferCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }

    async fn move_to(&self, view: ViewId, pos: Pos) {
        let (doc, mut cursors, tab_width, doc_decs, view_decs) = {
            let view_store = self.state.view_store.read().unwrap();
            let doc_store = self.state.document_store.read().unwrap();

            let Some(view) = view_store.get(&view) else {
                return;
            };

            let Some(&doc_id) = view.get::<DocumentId>() else {
                return;
            };
            let Some(doc) = doc_store.get(&doc_id) else {
                return;
            };

            let cursors = view.get::<ViewStoreTypes::Cursors>().cloned().unwrap_or_default();
            let tab_width =
                view.get::<ViewStoreTypes::TabWidth>().map(|&tab_width| *tab_width).unwrap_or(4);
            let doc_decs = doc.get::<DocumentStoreTypes::Decorations>().cloned();
            let view_decs = view.get::<ViewStoreTypes::Decorations>().cloned();

            (doc_id, cursors, tab_width, doc_decs, view_decs)
        };
        let (doc_decs, view_decs) = (doc_decs.as_ref(), view_decs.as_ref());

        let Ok(lines) = self.core_tx.lines(doc).await else {
            return;
        };
        // lines are one indexed.
        let y = pos.y.min(lines - 1);

        let stops = self.cursor_stops(doc, y, tab_width, doc_decs, view_decs).await;
        let x = stops.into_iter().rev().find(|&x| x <= pos.x).unwrap_or(0);

        // Moving to a specific location collapses all cursors.
        cursors.list.drain(1..);
        cursors.list[0] = Cursor::new(Pos::new(x, y), x);

        let mut view_store = self.state.view_store.write().unwrap();
        if let Some(view) = view_store.get_mut(&view) {
            view.insert(cursors.clone());
        }
        drop(view_store);

        if let Some(cursor) = cursors.list.first() {
            let _ = self.buffer_tx.send(BufferCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }

    async fn cursor_stops(
        &self, doc: DocumentId, y: usize, tab_width: usize, doc_decs: Option<&DocDecorations>,
        view_decs: Option<&ViewDecorations>,
    ) -> Vec<usize> {
        let Ok(line) = self.core_tx.get_line(doc, y).await else {
            return vec![0];
        };
        let Ok(start) = self.core_tx.get_line_start_byte(doc, y).await else {
            return vec![0];
        };

        let (layout, _) = render::layout(&line, start, tab_width, doc_decs, view_decs);

        if layout.cursor_stops.is_empty() { vec![0] } else { layout.cursor_stops }
    }
}
