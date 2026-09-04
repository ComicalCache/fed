use piece_table::Slice;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    protocols::view::ViewCommand,
    render,
    state::{
        DocumentId,
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
    buffer_tx: UnboundedSender<ViewCommand>,
}

impl CursorProtocol {
    pub fn new(
        state: State, rx: UnboundedReceiver<CursorCommand>, buffer_tx: UnboundedSender<ViewCommand>,
    ) -> Self {
        Self { state, rx, buffer_tx }
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
        let Some((Some(doc), mut cursors, tab_width, view_decs)) =
            self.state.with_view(view, |vm| {
                let doc = vm.get::<DocumentId>().cloned();
                let cursors = vm.get::<ViewStoreTypes::Cursors>().cloned().unwrap_or_default();
                let tab_width = vm.get::<ViewStoreTypes::TabWidth>().map(|&tw| *tw).unwrap_or(4);
                let view_decs = vm.get::<ViewStoreTypes::Decorations>().cloned();

                (doc, cursors, tab_width, view_decs)
            })
        else {
            return;
        };
        let Some(doc_decs) =
            self.state.with_doc(doc, |dm| dm.get::<DocumentStoreTypes::Decorations>().cloned())
        else {
            return;
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
                    let Some(lines) = self
                        .state
                        .with_doc(doc, |dm| {
                            dm.get::<DocumentStoreTypes::Document>()
                                .and_then(|d| Some(d.data.lines()))
                        })
                        .flatten()
                    else {
                        return;
                    };

                    if cursor.pos.y < lines.saturating_sub(1) {
                        cursor.pos.y += 1;
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

        self.state.with_view_mut(view, |vm| vm.insert(cursors.clone()));
        if let Some(cursor) = cursors.list.first() {
            let _ = self.buffer_tx.send(ViewCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }

    async fn move_to(&self, view: ViewId, pos: Pos) {
        let Some((Some(doc), mut cursors, tab_width, view_decs)) =
            self.state.with_view(view, |vm| {
                let doc = vm.get::<DocumentId>().cloned();
                let cursors = vm.get::<ViewStoreTypes::Cursors>().cloned().unwrap_or_default();
                let tab_width = vm.get::<ViewStoreTypes::TabWidth>().map(|&tw| *tw).unwrap_or(4);
                let view_decs = vm.get::<ViewStoreTypes::Decorations>().cloned();

                (doc, cursors, tab_width, view_decs)
            })
        else {
            return;
        };
        let Some(doc_decs) =
            self.state.with_doc(doc, |dm| dm.get::<DocumentStoreTypes::Decorations>().cloned())
        else {
            return;
        };
        let (doc_decs, view_decs) = (doc_decs.as_ref(), view_decs.as_ref());

        let Some(lines) = self
            .state
            .with_doc(doc, |dm| {
                dm.get::<DocumentStoreTypes::Document>().and_then(|d| Some(d.data.lines()))
            })
            .flatten()
        else {
            return;
        };

        // lines are one indexed.
        let y = pos.y.min(lines - 1);

        let stops = self.cursor_stops(doc, y, tab_width, doc_decs, view_decs).await;
        let x = stops.into_iter().rev().find(|&x| x <= pos.x).unwrap_or(0);

        // Moving to a specific location collapses all cursors.
        cursors.list.drain(1..);
        cursors.list[0] = Cursor::new(Pos::new(x, y), x);

        self.state.with_view_mut(view, |vm| vm.insert(cursors.clone()));
        if let Some(cursor) = cursors.list.first() {
            let _ = self.buffer_tx.send(ViewCommand::ScrollIfNeeded { view, pos: cursor.pos });
        }
    }

    async fn cursor_stops(
        &self, doc: DocumentId, y: usize, tab_width: usize, doc_decs: Option<&DocDecorations>,
        view_decs: Option<&ViewDecorations>,
    ) -> Vec<usize> {
        let Some((offset, line)) = self
            .state
            .with_doc(doc, |dm| {
                dm.get::<DocumentStoreTypes::Document>().and_then(|d| {
                    let start = d.data.get_line_start_byte(y);
                    let end = d.data.get_line_end_byte(y);

                    Some((start, d.data.slice(start..end)))
                })
            })
            .flatten()
        else {
            return vec![0];
        };

        let (layout, _) = render::layout(&line, offset, tab_width, doc_decs, view_decs);

        if layout.cursor_stops.is_empty() { vec![0] } else { layout.cursor_stops }
    }
}
