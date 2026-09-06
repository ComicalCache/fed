use std::collections::HashMap;

use crate::{newtype::newtype, state::document::DocumentId};

newtype!(ViewId, u64);

pub type ViewStore = HashMap<ViewId, ViewStoreEntry>;

#[derive(Default)]
pub struct ViewStoreEntry {
    pub doc: DocumentId,
    pub tab_width: types::TabWidth,
    pub scroll: types::Scroll,
    pub cursors: types::Cursors,
    pub mode: types::Mode,
    pub layout: types::Layout,
    pub decs: types::Decorations,
}

pub mod types {
    use std::sync::Arc;

    use rust_lapper::Lapper;

    use crate::{
        newtype::newtype,
        types::{Cursor, Decoration, Pos},
    };

    newtype!(TabWidth, usize);

    newtype!(Scroll, Pos);

    #[derive(Clone)]
    pub struct Cursors {
        pub list: Vec<Cursor>,
    }

    impl Default for Cursors {
        fn default() -> Self { Self { list: vec![Cursor::default()] } }
    }

    #[derive(Default, Clone, Copy, PartialEq, Eq)]
    pub enum Mode {
        #[default]
        Normal,
        Insert,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Layout {
        pub gutter: usize,
        pub mode_line: usize,
    }

    impl Default for Layout {
        fn default() -> Self { Self { gutter: 2, mode_line: 1 } }
    }

    #[derive(Clone)]
    pub struct Decorations {
        pub tree: Arc<Lapper<usize, Decoration>>,
    }

    impl Default for Decorations {
        fn default() -> Self { Self { tree: Arc::new(Lapper::new(vec![])) } }
    }
}
