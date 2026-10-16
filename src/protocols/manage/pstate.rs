use crate::state::{DocId, ViewId};

#[derive(Clone, Copy)]
pub enum ManageEntry {
    Doc(DocId),
    View(ViewId),
}

#[derive(Default)]
pub struct ManagePState {
    pub doc: DocId,
    pub view: ViewId,

    pub entries: Vec<ManageEntry>,
}
