use std::collections::HashMap;

use crate::newtype::newtype;

newtype!(DocumentId, u64);

pub type DocumentStore = HashMap<DocumentId, DocumentStoreEntry>;

#[derive(Default)]
pub struct DocumentStoreEntry {
    pub doc: types::Document,
    pub decs: types::Decorations,
}

pub mod types {
    use std::{path::PathBuf, sync::Arc};

    use piece_table::PieceTable;
    use rust_lapper::Lapper;

    use crate::{state::document::DocumentId, types::Decoration};

    #[derive(Default)]
    pub struct Document {
        pub path: Option<PathBuf>,

        pub data: PieceTable,
        pub modified: bool,
    }

    impl Document {
        pub fn new(path: Option<PathBuf>, data: PieceTable) -> Self {
            Self { path, data, modified: false }
        }
    }

    pub enum DocumentEvent {
        Created { id: DocumentId },
        Destroyed { id: DocumentId },
        Inserted { id: DocumentId, pos: usize, n: usize, str: String },
        Removed { id: DocumentId, pos: usize, n: usize, str: String },
        Saved { id: DocumentId, path: PathBuf, bytes_written: usize },
    }

    #[derive(Clone)]
    pub struct Decorations {
        pub tree: Arc<Lapper<usize, Decoration>>,
    }

    impl Default for Decorations {
        fn default() -> Self { Self { tree: Arc::new(Lapper::new(vec![])) } }
    }
}
