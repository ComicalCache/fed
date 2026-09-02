use std::collections::HashMap;

use crate::state::ViewId;

pub type LocalViewStore = HashMap<ViewId, LocalViewData>;

pub struct LocalViewData {
    pub offset: usize,
    pub lines: Vec<String>,
}
