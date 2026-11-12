use crate::state::DocId;

#[derive(Default)]
pub struct KeybindPState {
    pub doc: Option<DocId>,
}
