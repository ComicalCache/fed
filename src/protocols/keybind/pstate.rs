use crate::state::DocId;
use crate::state::ViewId;

#[derive(Default)]
pub struct KeybindPState {
    pub doc: DocId,
    pub view: ViewId,
}
