use std::collections::HashMap;

use fed_core::DocumentId;

use crate::type_map::TypeMap;

pub type DocumentStore = HashMap<DocumentId, TypeMap>;

pub mod types {
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
}
