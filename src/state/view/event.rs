use crate::state::{ViewId, ViewStoreTypes};

#[derive(Clone)]
pub enum Event {
    CursorMoved { view: ViewId },
    CursorsChanged { view: ViewId },
    ModeChanged { id: ViewId, mode: ViewStoreTypes::Mode },
}
