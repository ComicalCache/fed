mod cursors;
mod decorations;
mod layout;
mod mode;
mod scroll;
mod tab_width;

pub mod types {
    pub use crate::state::view::{
        cursors::Cursors, decorations::Decorations, layout::Layout, mode::Mode, scroll::Scroll,
        tab_width::TabWidth,
    };
}

use std::collections::HashMap;

use crate::{newtype::newtype, render::ModeLineConfig, state::document::DocumentId};

newtype!(ViewId, u64);

pub type ViewStore = HashMap<ViewId, ViewStoreEntry>;

#[derive(Default)]
pub struct ViewStoreEntry {
    pub doc: DocumentId,
    pub tab_width: types::TabWidth,
    pub scroll: types::Scroll,
    pub cursors: types::Cursors,
    pub mode: types::Mode,
    pub mode_line_config: ModeLineConfig,
    pub layout: types::Layout,
    pub decs: types::Decorations,
}
