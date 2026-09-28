use std::sync::{Arc, RwLock};

use crate::{
    debug_panic::debug_panic,
    protocols::view::{
        layers::{CursorLayer, RulerLayer, SelectionLayer},
        store::LocalViewStore,
    },
    render::{self, Cell, Layer, Renderer, Viewport, WindowId},
    state::{State, ViewId},
    types::Pos,
};

pub struct ViewRenderer {
    view: ViewId,

    layers: [Box<dyn Layer>; 3],

    store: Arc<RwLock<LocalViewStore>>,
}

impl ViewRenderer {
    pub fn new(view: ViewId, store: Arc<RwLock<LocalViewStore>>) -> Self {
        Self {
            view,
            layers: [
                Box::new(SelectionLayer {}),
                Box::new(RulerLayer {}),
                Box::new(CursorLayer {}),
            ],
            store,
        }
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
                    let mut cell = Cell::default();
                    cell.face = state.theme.default;

                    viewport.set(Pos::new(x, y), cell);
                }
            }

            return;
        };
        drop(store);

        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, self.view)
        else {
            debug_panic!();
            return;
        };

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

            let (vom, next_offset) =
                render::layout_vom(line, offset, &decs, &vse.layout.replacements, vse.tab_width);
            let cells =
                render::layout_cells(line, offset, &decs, &vse.layout.replacements, vse.tab_width);
            offset = next_offset;

            let mut row = vec![Cell::default(); width];

            let mut x = 0;
            let mut visual_x = 0;
            for cell in cells {
                if visual_x < vse.scroll.x {
                    visual_x += 1;
                    continue;
                }

                if x >= width {
                    break;
                }

                let mut face = if state.mini_buffer_store.view == self.view {
                    state.theme.mini_buffer
                } else {
                    state.theme.default
                };
                face.merge(cell.face);

                if visual_x == vse.scroll.x && cell.width == 0 {
                    // A wide char's first section is off-screen.
                    row[x] = Cell::new(" ".to_string(), 1, face);
                } else if x + cell.width > width {
                    // A wide char's trailing section is off-screen.
                    row[x] = Cell::new(" ".to_string(), 1, face);
                } else {
                    row[x] = Cell::new(cell.ch, cell.width, face);
                }

                x += 1;
                visual_x += 1;
            }

            // Undrawn tail of line.
            while x < width {
                let face = if state.mini_buffer_store.view == self.view {
                    state.theme.mini_buffer
                } else {
                    state.theme.default
                };

                row[x] = Cell::new(" ".to_string(), 1, face);
                x += 1;
            }

            for layer in &self.layers {
                layer.apply(state, &mut row, vse.scroll.x, &vom, vse, dse);
            }

            for (idx, cell) in row.into_iter().enumerate() {
                viewport.set(Pos::new(idx, y), cell);
            }

            lines_drawn += 1;
        }

        // Undrawn trailing lines.
        let face = if state.mini_buffer_store.view == self.view {
            state.theme.mini_buffer
        } else {
            state.theme.default
        };
        for y in lines_drawn..height {
            let mut row = vec![Cell::new(" ".to_string(), 1, face); width];

            for layer in &self.layers {
                layer.apply(state, &mut row, vse.scroll.x, &[], vse, dse);
            }

            for (x, cell) in row.into_iter().enumerate() {
                viewport.set(Pos::new(x, y), cell);
            }
        }
    }
}
