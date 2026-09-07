use std::sync::Arc;

use rust_lapper::Lapper;

use crate::types::Decoration;

#[derive(Clone)]
pub struct Decorations {
    pub tree: Arc<Lapper<usize, Decoration>>,
}

impl Default for Decorations {
    fn default() -> Self { Self { tree: Arc::new(Lapper::new(vec![])) } }
}
