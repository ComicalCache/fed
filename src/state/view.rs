use std::collections::HashMap;

use crate::{newtype::newtype, type_map::TypeMap};

newtype!(ViewId, u64);

pub type ViewStore = HashMap<ViewId, TypeMap>;

pub mod types {
    use std::sync::Arc;

    use rust_lapper::Lapper;

    use crate::{
        newtype::newtype,
        types::{Cursor, Decoration, Pos},
    };

    newtype!(TabWidth, usize);

    newtype!(Scroll, Pos);

    #[derive(Default, Clone)]
    pub struct Cursors {
        pub list: Vec<Cursor>,
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
