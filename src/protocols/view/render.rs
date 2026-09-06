use std::sync::{Arc, RwLock};

use crate::{
    protocols::view::store::LocalViewStore,
    render::{self, Cell, Renderer, Viewport, WindowId},
    state::{DocumentId, State, ViewId},
    types::{Face, Pos},
};

pub struct ViewRenderer {
    doc: DocumentId,
    view: ViewId,

    store: Arc<RwLock<LocalViewStore>>,

    state: State,
}

impl ViewRenderer {
    pub fn new(
        doc: DocumentId, view: ViewId, store: Arc<RwLock<LocalViewStore>>, state: State,
    ) -> Self {
        Self { doc, view, store, state }
    }
}

impl Renderer for ViewRenderer {
    fn render(&self, viewport: &mut Viewport, _: WindowId) {
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

        let Some((cursors, scroll, tab_width, view_decs)) = self.state.with_view(self.view, |vm| {
            (vm.cursors.clone(), vm.scroll, vm.tab_width, vm.decs.clone())
        }) else {
            return;
        };
        let Some(doc_decs) = self.state.with_doc(self.doc, |dm| dm.decs.clone()) else { return };

        let mut lines_drawn = 0;
        let mut offset = entry.offset;
        for (y, line) in entry.lines.iter().enumerate() {
            if y >= viewport_height {
                break;
            }

            let (layout, next_offset) =
                render::layout(line, offset, tab_width, &doc_decs, &view_decs);
            offset = next_offset;

            let mut x = 0;
            let mut visual_x = 0;
            for cell in layout.cells {
                if visual_x < scroll.x {
                    visual_x += 1;
                    continue;
                }

                if x >= viewport_width {
                    break;
                }

                let mut face = cell.face;
                if cursors
                    .list
                    .iter()
                    .any(|cursor| cursor.pos.y == y + scroll.y && cursor.pos.x == visual_x)
                {
                    face.reverse = Some(true);
                }

                if visual_x == scroll.x && cell.width == 0 {
                    // A wide char's first section is off-screen.
                    viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                } else if x + cell.width > viewport_width {
                    // A wide char's trailing section is off-screen.
                    viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                } else {
                    viewport.set(Pos::new(x, y), Cell::new(cell.ch, cell.width == 0, face));
                }

                x += 1;
                visual_x += 1;
            }

            // Undrawn tail of line.
            while x < viewport_width {
                let mut face = Face::default();
                if cursors
                    .list
                    .iter()
                    .any(|cursor| cursor.pos.y == y + scroll.y && cursor.pos.x == visual_x)
                {
                    face.reverse = Some(true);
                }

                viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                x += 1;
                visual_x += 1;
            }

            lines_drawn += 1;
        }

        // Undrawn trailing lines.
        for y in lines_drawn..viewport_height {
            for x in 0..viewport_width {
                let mut face = Face::default();
                if cursors
                    .list
                    .iter()
                    .any(|cursor| cursor.pos.y == y + scroll.y && cursor.pos.x == x + scroll.x)
                {
                    face.reverse = Some(true);
                }

                viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
            }
        }
    }
}
