use crate::state::{DocId, ViewId};

#[derive(Clone, Copy)]
pub enum Entry {
    Doc(DocId),
    View(ViewId),
}
