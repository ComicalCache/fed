use fed_core::{CoreCommandSender, DocumentId};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    protocols::buffer::BufferCommand,
    state::{State, ViewId, ViewStoreTypes},
    types::Direction,
};

pub enum CursorCommand {
    Move { view: ViewId, direction: Direction },
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
                    cursor.y = cursor.y.saturating_sub(1);

                    let Ok(line) = self.core_tx.get_line(doc, cursor.y).await else {
                        return;
                    };

                    let visible_len =
                        line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();

                    if cursor.x > visible_len {
                        cursor.x = visible_len;
                    }
                }
                Direction::Down => {
                    if let Ok(lines) = self.core_tx.lines(doc).await
                        && cursor.y < lines
                    {
                        cursor.y += 1;
                    }

                    let Ok(line) = self.core_tx.get_line(doc, cursor.y).await else {
                        return;
                    };

                    let visible_len =
                        line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();

                    if cursor.x > visible_len {
                        cursor.x = visible_len;
                    }
                }
                Direction::Left => cursor.x = cursor.x.saturating_sub(1),
                Direction::Right => {
                    if let Ok(line) = self.core_tx.get_line(doc, cursor.y).await {
                        let visible_len =
                            line.trim_end_matches(&['\n', '\r'][..]).graphemes(true).count();

                        if cursor.x < visible_len {
                            cursor.x += 1;
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
            let _ = self.buffer_tx.send(BufferCommand::ScrollIfNeeded { view, pos: *cursor });
        }
    }
}
