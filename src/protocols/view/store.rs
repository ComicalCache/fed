use std::collections::HashMap;

use crate::state::ViewId;

pub type LocalViewStore = HashMap<ViewId, LocalViewData>;

#[derive(Clone)]
pub struct LocalViewData {
    pub offset: usize,
    pub lines: Vec<String>,
}
