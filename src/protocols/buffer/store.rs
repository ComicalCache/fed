use std::collections::HashMap;

use crate::state::ViewId;

pub type BufferStore = HashMap<ViewId, Vec<String>>;
