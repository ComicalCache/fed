use std::sync::{Arc, RwLock};

use fed_core::DocumentId;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    protocols::buffer::store::BufferStore,
    render::{Cell, Renderer, Viewport, WindowId},
    state::{State, ViewId, ViewStoreTypes},
    types::{Face, Pos},
};

pub struct BufferRenderer {
    doc: DocumentId,
    view: ViewId,

    store: Arc<RwLock<BufferStore>>,

    state: State,
}

impl BufferRenderer {
    pub fn new(
        doc: DocumentId, view: ViewId, store: Arc<RwLock<BufferStore>>, state: State,
    ) -> Self {
        Self { doc, view, store, state }
    }
}

impl Renderer for BufferRenderer {
    fn render(&self, viewport: &mut Viewport, _: WindowId) {
        // TODO: use properties for replacements, faces, etc...

        let viewport_width = viewport.width();
        let viewport_height = viewport.height();

        let store = self.store.read().unwrap();

        let Some(entry) = store.get(&self.view) else {
            for y in 0..viewport_height {
                for x in 0..viewport_width {
                    viewport.set(Pos::new(x, y), Cell::default());
                }
            }

            return;
        };

        let (tab_width, cursors, scroll) = {
            let view_store = self.state.view_store.read().unwrap();
            let view = view_store.get(&self.view);

            let tab_width = view
                .and_then(|view| view.get::<ViewStoreTypes::TabWidth>())
                .map(|&width| *width)
                .unwrap_or(4);
            let cursors = view.and_then(|view| view.get::<ViewStoreTypes::Cursors>()).cloned();
            let scroll = view
                .and_then(|view| view.get::<ViewStoreTypes::Scroll>())
                .map(|&scroll| scroll)
                .unwrap_or_default();

            (tab_width, cursors, scroll)
        };

        let mut lines_drawn = 0;
        for (y, line) in entry.iter().enumerate() {
            if y >= viewport_height {
                break;
            }

            let doc_y = y + scroll.y;

            let mut x = 0;
            let mut visual_x = 0;
            for ch in line.graphemes(true) {
                if x >= viewport_width {
                    break;
                }

                let ch_width = if ch == "\t" {
                    tab_width - (visual_x % tab_width)
                } else if ch == "\n" {
                    // "\n".width() == 1!
                    0
                } else {
                    ch.width()
                };

                if ch_width == 0 {
                    continue;
                }

                let start_col = visual_x;

                visual_x += ch_width;
                if visual_x <= scroll.x {
                    continue;
                }

                let mut face = Face::default();
                if let Some(cursors) = &cursors
                    && cursors
                        .list
                        .iter()
                        .any(|cursor| cursor.pos.y == doc_y && cursor.pos.x == start_col)
                {
                    face.reverse = Some(true);
                }

                if ch == "\t" {
                    let visible_spaces = visual_x.saturating_sub(scroll.x.max(start_col));
                    for _ in 0..visible_spaces {
                        if x >= viewport_width {
                            break;
                        }

                        viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                        x += 1;
                    }

                    continue;
                }

                // A wide char's first section is off-screen.
                if start_col < scroll.x {
                    viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                    x += 1;

                    continue;
                }

                // A wide char's trailing section is off-screen.
                if x + ch_width > viewport_width {
                    while x < viewport_width {
                        viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                        x += 1;
                    }

                    break;
                }

                viewport.set(Pos::new(x, y), Cell::new(ch.to_string(), false, face));
                x += 1;

                // Trailing wide cells.
                for _ in 1..ch_width {
                    if x >= viewport_width {
                        break;
                    }

                    viewport.set(Pos::new(x, y), Cell::new(String::new(), true, face));
                    x += 1;
                }
            }

            // Undrawn tail of line.
            for x in x..viewport_width {
                let mut face = Face::default();
                if let Some(cursors) = &cursors
                    && cursors
                        .list
                        .iter()
                        .any(|cursor| cursor.pos.y == doc_y && cursor.pos.x == visual_x)
                {
                    face.reverse = Some(true);
                }

                viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                visual_x += 1;
            }

            lines_drawn += 1;
        }

        // Undrawn trailing lines.
        for y in lines_drawn..viewport_height {
            for x in 0..viewport_width {
                viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, Face::default()));
            }
        }
    }
}
