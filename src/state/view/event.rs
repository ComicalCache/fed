use crate::state::ViewId;
use crate::state::ViewStoreTypes;

#[derive(Clone)]
#[allow(unused)]
pub enum Event {
    Created { view: ViewId },
    Destroyed { view: ViewId },
    CursorsMoved { view: ViewId },
    CursorsChanged { view: ViewId },
    ModeChanged { view: ViewId, mode: ViewStoreTypes::Mode },
    Resized { view: ViewId },
}
