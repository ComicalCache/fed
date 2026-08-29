use std::collections::HashMap;

use crate::{state::ViewId, types::Pos};

pub type BufferStore = HashMap<ViewId, BufferStoreEntry>;

pub struct BufferStoreEntry {
    pub lines: Vec<String>,
    pub scroll: Pos,
}
