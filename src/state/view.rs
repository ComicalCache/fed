mod cursors;
mod decorations;
mod event;
mod layout;
mod mode;
mod scroll;
mod view_cache;

pub mod types {
    pub use crate::state::view::{
        cursors::Cursors,
        decorations::{DecorationId, Decorations},
        event::Event,
        layout::{Layout, Replacements, TabWidth, ViewLayout},
        mode::Mode,
        scroll::Scroll,
        view_cache::ViewCache,
    };
}

use std::collections::HashMap;

use crate::{
    state::{DocStoreEntry, ViewStoreTypes},
    types::{Pos, Rect, render::ModeLineConfig},
    util::newtype,
};

newtype!(ViewId, usize);

pub type ViewStore = HashMap<ViewId, ViewStoreEntry>;

pub struct ViewStoreEntry {
    pub rect: Rect,

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

    pub fn layout(&self, dse: &DocStoreEntry) -> types::ViewLayout {
        let gutter_width = self.layout.gutter_width(dse.doc.data.lines()).min(self.rect.width);
        let mode_line_height = self.layout.mode_line.min(self.rect.height);
        let content_height = self.rect.height.saturating_sub(mode_line_height);

        types::ViewLayout {
            gutter: Rect::new(self.rect.pos, gutter_width, content_height),
            mode_line: Rect::new(
                self.rect.pos + Pos::new(0, content_height),
                self.rect.width,
                mode_line_height,
            ),
            content: Rect::new(
                self.rect.pos + Pos::new(gutter_width, 0),
                self.rect.width.saturating_sub(gutter_width),
                content_height,
            ),
        }
    }
}

impl Default for ViewStoreEntry {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            modes: vec![ViewStoreTypes::Mode::Normal],
            view_cache: ViewStoreTypes::ViewCache::default(),
            layout: ViewStoreTypes::Layout::default(),
            mode_line_config: ModeLineConfig::default(),
            scroll: ViewStoreTypes::Scroll::default(),
            cursors: ViewStoreTypes::Cursors::default(),
            decs: ViewStoreTypes::Decorations::default(),
        }
    }
}
