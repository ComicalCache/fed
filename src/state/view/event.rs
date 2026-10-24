use crate::state::ViewId;
use crate::state::ViewStoreTypes;

#[derive(Clone)]
#[allow(unused)]
pub enum Event {
    Created { id: ViewId },
    Destroyed { id: ViewId },
    CursorsMoved { view: ViewId },
    CursorsChanged { view: ViewId },
    ModeChanged { id: ViewId, mode: ViewStoreTypes::Mode },
    Resized { id: ViewId },
}
