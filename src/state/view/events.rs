use crate::state::ViewId;

#[derive(Clone)]
pub enum ViewEvent {
    CursorMoved { view: ViewId },
}
