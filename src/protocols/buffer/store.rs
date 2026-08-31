use std::collections::HashMap;

use crate::state::ViewId;

pub type BufferStore = HashMap<ViewId, BufferData>;

pub struct BufferData {
    pub offset: usize,
    pub lines: Vec<String>,
}
