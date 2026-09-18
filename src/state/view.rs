mod cursors;
mod decorations;
mod layout;
mod mode;
mod scroll;
mod tab_width;

pub mod types {
    pub use crate::state::view::{
        cursors::Cursors,
        decorations::{Decorations, ViewDecoration},
        layout::Layout,
        mode::Mode,
        scroll::Scroll,
        tab_width::TabWidth,
    };
}

use std::collections::HashMap;

use crate::{newtype::newtype, render::ModeLineConfig};

newtype!(ViewId, usize);

pub type ViewStore = HashMap<ViewId, ViewStoreEntry>;

#[derive(Default)]
pub struct ViewStoreEntry {
    pub mode: types::Mode,

    pub layout: types::Layout,
    pub mode_line_config: ModeLineConfig,

    pub tab_width: types::TabWidth,
    pub scroll: types::Scroll,
    pub cursors: types::Cursors,

    pub decs: types::Decorations,
}
