use std::collections::HashMap;

use crate::state::DocId;
use crate::state::ViewId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ManageEntry {
    Doc(DocId),
    View(ViewId),
}

#[derive(Default)]
pub struct ManagePState {
    pub doc: DocId,
    pub view: ViewId,

    pub entries: Vec<ManageEntry>,
    pub line_map: HashMap<DocId, usize>,
}
