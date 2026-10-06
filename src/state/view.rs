mod cursors;
mod decorations;
mod event;
mod layout;
mod mode;
mod scroll;
mod tab_width;
mod view_cache;

pub mod types {
    pub use crate::state::view::{
        cursors::Cursors,
        decorations::{DecorationId, Decorations},
        event::Event,
        layout::{Layout, Replacements},
        mode::Mode,
        scroll::Scroll,
        tab_width::TabWidth,
        view_cache::ViewCache,
    };
}

use std::collections::HashMap;

use crate::{newtype::newtype, render::ModeLineConfig};

newtype!(ViewId, usize);

pub type ViewStore = HashMap<ViewId, ViewStoreEntry>;

pub struct ViewStoreEntry {
    pub modes: Vec<types::Mode>,

    pub view_cache: types::ViewCache,

    pub layout: types::Layout,
    pub mode_line_config: ModeLineConfig,

    pub scroll: types::Scroll,
    pub cursors: types::Cursors,

    pub decs: types::Decorations,
}

impl ViewStoreEntry {
    pub fn mode(&self) -> types::Mode { *self.modes.last().unwrap() }
}

impl Default for ViewStoreEntry {
    fn default() -> Self {
        Self {
            modes: vec![types::Mode::Normal],
            view_cache: Default::default(),
            layout: Default::default(),
            mode_line_config: Default::default(),
            scroll: Default::default(),
            cursors: Default::default(),
            decs: Default::default(),
        }
    }
}
