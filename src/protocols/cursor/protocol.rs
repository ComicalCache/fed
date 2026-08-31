use fed_core::{CoreCommandSender, DocumentId};
use unicode_segmentation::UnicodeSegmentation;

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
        let (doc, mut cursors) = {
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

            (doc, cursors)
        };

        for cursor in &mut cursors.list {
            match direction {
                Direction::Up => {
                    cursor.pos.y = cursor.pos.y.saturating_sub(1);

                    let Ok(line) = self.core_tx.get_line(doc, cursor.pos.y).await else {
                        return;
                    };

                    let visible_len =
                        line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();

                    if cursor.pref_x > visible_len {
                        cursor.pos.x = visible_len;
                    } else {
                        cursor.pos.x = cursor.pref_x;
                    }
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

                    let visible_len =
                        line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();

                    if cursor.pref_x > visible_len {
                        cursor.pos.x = visible_len;
                    } else {
                        cursor.pos.x = cursor.pref_x;
                    }
                }
                Direction::Left => {
                    cursor.pos.x = cursor.pos.x.saturating_sub(1);
                    cursor.pref_x = cursor.pos.x;
                }
                Direction::Right => {
                    if let Ok(line) = self.core_tx.get_line(doc, cursor.pos.y).await {
                        let visible_len =
                            line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();

                        if cursor.pos.x < visible_len {
                            cursor.pos.x += 1;
                            cursor.pref_x = cursor.pos.x;
                        }
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
        let (doc, mut cursors) = {
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

            (doc, cursors)
        };

        let Ok(lines) = self.core_tx.lines(doc).await else {
            return;
        };
        // lines are one indexed.
        let y = pos.y.min(lines - 1);

        let Ok(line) = self.core_tx.get_line(doc, y).await else {
            return;
        };
        let visible_len = line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();
        let x = pos.x.min(visible_len);

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
}
