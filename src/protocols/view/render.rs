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
}

impl ViewRenderer {
    pub fn new(doc: DocumentId, view: ViewId, store: Arc<RwLock<LocalViewStore>>) -> Self {
        Self { doc, view, store }
    }
}

impl Renderer for ViewRenderer {
    fn render(&self, state: &State, viewport: &mut Viewport, _: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        let store = self.store.read().unwrap();

        let Some(lvd) = store.get(&self.view).cloned() else {
            for y in 0..height {
                for x in 0..width {
                    viewport.set(Pos::new(x, y), Cell::default());
                }
            }

            return;
        };

        drop(store);

        let Some((vse, dse)) = state.vse_and_dse(self.view) else { return };

        let cursors = vse.cursors.clone();
        let scroll = vse.scroll;
        let tab_width = vse.tab_width;

        let start = lvd.offset;
        let end = start + lvd.lines.iter().take(height).map(|l| l.len()).sum::<usize>();

        let mut decs = Vec::new();
        dse.decs.range(start, end, &mut decs);
        vse.decs.range(start, end, &mut decs);

        let mut lines_drawn = 0;
        let mut offset = lvd.offset;
        for (y, line) in lvd.lines.iter().enumerate() {
            if y >= height {
                break;
            }

            let (layout, next_offset) = render::layout(line, offset, tab_width, &decs);
            offset = next_offset;

            let mut cursor_xs = Vec::new();
            for vo in &layout.visual_offset_mapping {
                if cursors.list.iter().any(|c| c.offset == vo.offset) {
                    cursor_xs.push(vo.visual_x);
                }
            }

            let mut x = 0;
            let mut visual_x = 0;
            for cell in layout.cells {
                if visual_x < scroll.x {
                    visual_x += 1;
                    continue;
                }

                if x >= width {
                    break;
                }

                let mut face = cell.face;
                if cursor_xs.contains(&visual_x) {
                    face.reverse = Some(true);
                }

                if visual_x == scroll.x && cell.width == 0 {
                    // A wide char's first section is off-screen.
                    viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                } else if x + cell.width > width {
                    // A wide char's trailing section is off-screen.
                    viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                } else {
                    viewport.set(Pos::new(x, y), Cell::new(cell.ch, cell.width == 0, face));
                }

                x += 1;
                visual_x += 1;
            }

            // Undrawn tail of line.
            while x < width {
                let mut face = Face::default();
                if cursor_xs.contains(&visual_x) {
                    face.reverse = Some(true);
                }

                viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, face));
                x += 1;
                visual_x += 1;
            }

            lines_drawn += 1;
        }

        // Undrawn trailing lines.
        for y in lines_drawn..height {
            for x in 0..width {
                viewport.set(Pos::new(x, y), Cell::new(" ".to_string(), false, Face::default()));
            }
        }
    }
}
