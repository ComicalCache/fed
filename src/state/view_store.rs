use std::collections::HashMap;

use crate::{newtype::newtype, type_map::TypeMap};

newtype!(ViewId, u64);

pub type ViewStore = HashMap<ViewId, TypeMap>;

pub mod types {
    use crate::{newtype::newtype, types::Pos};

    newtype!(TabWidth, usize);

    #[derive(Default, Clone)]
    pub struct Cursors {
        pub list: Vec<Pos>,
    }

    #[derive(Default, Clone, Copy, PartialEq, Eq)]
    pub enum Mode {
        #[default]
        Normal,
    }
}
