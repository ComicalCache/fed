use fed_core::{CoreCommandSender, DocumentId};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    protocols::buffer::BufferCommand,
    state::{State, ViewId, ViewStoreTypes},
    types::{Cursor, Direction, Pos},
};

pub enum CursorCommand {
    Move { view: ViewId, direction: Direction },
    MoveTo { view: ViewId, pos: Pos },
}

pub struct CursorProtocol {
    state: State,

    rx: flume::Receiver<CursorCommand>,
    buffer_tx: flume::Sender<BufferCommand>,
    core_tx: CoreCommandSender,
}

impl CursorProtocol {
    pub fn new(
        state: State, rx: flume::Receiver<CursorCommand>, buffer_tx: flume::Sender<BufferCommand>,
        core_tx: CoreCommandSender,
    ) -> Self {
        Self { state, rx, buffer_tx, core_tx }
    }

    pub async fn run(&mut self) {
        while let Ok(cmd) = self.rx.recv_async().await {
            match cmd {
                CursorCommand::Move { view, direction } => self.r#move(view, direction).await,
                CursorCommand::MoveTo { view, pos } => self.move_to(view, pos).await,
            }
        }
    }

    async fn r#move(&self, view: ViewId, direction: Direction) {
        let (doc, mut cursors, tab_width) = {
            let view_store = self.state.view_store.read().unwrap();
            let Some(view) = view_store.get(&view) else {
                return;
            };

            let Some(&doc) = view.get::<DocumentId>() else {
                return;
            };
            let Some(cursors) = view.get::<ViewStoreTypes::Cursors>().cloned() else {
                return;
            };
            let tab_width =
                view.get::<ViewStoreTypes::TabWidth>().map(|&tab_width| *tab_width).unwrap_or(4);

            (doc, cursors, tab_width)
        };

        for cursor in &mut cursors.list {
            match direction {
                Direction::Up => {
                    cursor.pos.y = cursor.pos.y.saturating_sub(1);

                    let Ok(line) = self.core_tx.get_line(doc, cursor.pos.y).await else {
                        return;
                    };

                    let cols = Self::visual_cols(&line, tab_width);
                    cursor.pos.x =
                        cols.into_iter().rev().find(|&x| x <= cursor.pref_x).unwrap_or(0);
                }
                Direction::Down => {
                    if let Ok(lines) = self.core_tx.lines(doc).await
                        // lines are one indexed.
                        && cursor.pos.y < lines - 1
                    {
                        cursor.pos.y += 1;
                    }

                    let Ok(line) = self.core_tx.get_line(doc, cursor.pos.y).await else {
                        return;
                    };

                    let cols = Self::visual_cols(&line, tab_width);
                    cursor.pos.x =
                        cols.into_iter().rev().find(|&x| x <= cursor.pref_x).unwrap_or(0);
                }
                Direction::Left => {
                    let Ok(line) = self.core_tx.get_line(doc, cursor.pos.y).await else {
                        return;
                    };

                    let cols = Self::visual_cols(&line, tab_width);
                    cursor.pos.x = cols.into_iter().rev().find(|&x| x < cursor.pos.x).unwrap_or(0);
                    cursor.pref_x = cursor.pos.x;
                }
                Direction::Right => {
                    let Ok(line) = self.core_tx.get_line(doc, cursor.pos.y).await else {
                        return;
                    };

                    let cols = Self::visual_cols(&line, tab_width);
                    if let Some(x) = cols.into_iter().find(|&x| x > cursor.pos.x) {
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
        let (doc, mut cursors, tab_width) = {
            let view_store = self.state.view_store.read().unwrap();
            let Some(view) = view_store.get(&view) else {
                return;
            };

            let Some(&doc) = view.get::<DocumentId>() else {
                return;
            };
            let Some(cursors) = view.get::<ViewStoreTypes::Cursors>().cloned() else {
                return;
            };
            let tab_width =
                view.get::<ViewStoreTypes::TabWidth>().map(|&tab_width| *tab_width).unwrap_or(4);

            (doc, cursors, tab_width)
        };

        let Ok(lines) = self.core_tx.lines(doc).await else {
            return;
        };
        // lines are one indexed.
        let y = pos.y.min(lines - 1);

        let Ok(line) = self.core_tx.get_line(doc, y).await else {
            return;
        };

        let cols = Self::visual_cols(&line, tab_width);
        let x = cols.into_iter().rev().find(|&x| x <= pos.x).unwrap_or(0);

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

    fn visual_cols(line: &str, tab_width: usize) -> Vec<usize> {
        let mut cols = vec![0];

        let mut x = 0;
        for ch in line.trim_end_matches(&['\n', '\r'][..]).graphemes(true) {
            let ch_width = if ch == "\t" { tab_width - (x % tab_width) } else { ch.width() };
            x += ch_width;

            cols.push(x);
        }

        cols
    }
}
