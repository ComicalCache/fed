use std::collections::HashMap;

use crate::{state::ViewId, types::render::WindowId};

#[derive(Default)]
pub struct DocViewPState {
    pub revision: u64,
    pub pending: HashMap<WindowId, (u64, Vec<ViewId>)>,
}
