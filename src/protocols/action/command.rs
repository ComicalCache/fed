use tokio::sync::oneshot;

use crate::{
    state::{DocId, DocStoreTypes, ViewId, ViewStoreTypes},
    types::{Motion, Pos},
};

pub enum ActionCommand {
    MoveCursors { view: ViewId, motion: Motion, move_anchor: bool },
    MoveCursorToPos { view: ViewId, pos: Pos, move_anchor: bool },
    CreateCursorAtPos { view: ViewId, pos: Pos },
    CreateCursorsAtOffset { view: ViewId, offsets: Vec<usize> },
    RemoveCursors { view: ViewId, offsets: Vec<usize> },

    StartCommit { doc: DocId },
    EndCommit { doc: DocId },

    Undo { view: ViewId },
    HotRedo { view: ViewId },

    Saved { doc: DocId },

    Insert { view: ViewId, text: String },
    InsertAt { view: ViewId, text: String, offset: usize },
    Backspace { view: ViewId },
    Delete { view: ViewId },
    Remove { view: ViewId, offset: usize, len: usize },

    SetDocMode { doc: DocId, mode: DocStoreTypes::Mode },
    PushViewMode { view: ViewId, mode: ViewStoreTypes::Mode },
    PopViewMode { view: ViewId },

    CanQuit { tx: oneshot::Sender<Result<(), String>> },
}
