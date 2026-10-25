use std::path::PathBuf;

use crate::state::DocId;
use crate::state::DocStoreTypes;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Decorator;
use crate::types::Motion;
use crate::types::Pos;
use crate::types::render::WindowId;

#[allow(unused)]
pub enum ActionCmd {
    MoveCursors { view: ViewId, motion: Motion, move_anchor: bool },
    MoveCursorToPos { view: ViewId, pos: Pos, move_anchor: bool },
    CreateCursorAtPos { view: ViewId, pos: Pos },
    CreateCursorsAtOffsets { view: ViewId, offsets: Vec<usize> },
    RemoveCursors { view: ViewId, offsets: Vec<usize> },
    ResetCursorAnchors { view: ViewId },

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

    SetDocPath { doc: DocId, path: Option<PathBuf> },

    SetDocMode { doc: DocId, mode: DocStoreTypes::Mode },
    PushViewMode { view: ViewId, mode: ViewStoreTypes::Mode },
    PopViewMode { view: ViewId },

    SetDocDecorator { doc: DocId, decoration: DocStoreTypes::DecorationId, dec: Box<dyn Decorator> },
    SetViewDecorator { view: ViewId, decoration: ViewStoreTypes::DecorationId, dec: Box<dyn Decorator> },
    RemoveDocDecorator { doc: DocId, decoration: DocStoreTypes::DecorationId },
    RemoveViewDecorator { view: ViewId, decoration: ViewStoreTypes::DecorationId },

    SetActiveWindow { window: Option<WindowId> },
}
