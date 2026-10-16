use crate::{
    state::State,
    types::{
        Pos,
        render::{self, Cell, Layer, Renderer, Viewport, WindowId},
    },
    util::debug_panic,
};

pub struct ViewRenderer {
    layers: Vec<Box<dyn Layer>>,
}

impl ViewRenderer {
    pub fn new(layers: Vec<Box<dyn Layer>>) -> Self { Self { layers } }
}

impl Renderer for ViewRenderer {
    fn render(&self, s: &State, viewport: &mut Viewport, window: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let Some(view) = s.index.window_to_view(window) else {
            debug_panic!();
            return;
        };

        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return;
        };

        let start = vse.view_cache.offset;
        let end = start + vse.view_cache.lines.iter().take(height).map(String::len).sum::<usize>();

        let mut decs = Vec::new();
        dse.decs.range(start, end, &mut decs);
        vse.decs.range(start, end, &mut decs);

        let mut lines_drawn = 0;
        let mut offset = vse.view_cache.offset;
        for (y, line) in vse.view_cache.lines.iter().enumerate() {
            if y >= height {
                break;
            }

            let (vom, next_offset) = render::layout_vom(
                line,
                offset,
                &decs,
                &vse.layout.replacements,
                vse.layout.tab_width,
            );
            let cells = render::layout_cells(
                line,
                offset,
                &decs,
                &vse.layout.replacements,
                vse.layout.tab_width,
            );
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

                let mut face = if s.p.mp.view == view { s.theme.mp } else { s.theme.default };
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
                let face = if s.p.mp.view == view { s.theme.mp } else { s.theme.default };

                row[x] = Cell::new(" ".to_string(), 1, face);
                x += 1;
            }

            for layer in &self.layers {
                layer.apply(s, &mut row, vse.scroll.y + y, vse.scroll, &vom, vse, dse);
            }

            for (idx, cell) in row.into_iter().enumerate() {
                viewport.set(Pos::new(idx, y), cell);
            }

            lines_drawn += 1;
        }

        // Undrawn trailing lines.
        let face = if s.p.mp.view == view { s.theme.mp } else { s.theme.default };
        for y in lines_drawn..height {
            let mut row = vec![Cell::new(" ".to_string(), 1, face); width];

            for layer in &self.layers {
                layer.apply(s, &mut row, vse.scroll.y + y, vse.scroll, &[], vse, dse);
            }

            for (x, cell) in row.into_iter().enumerate() {
                viewport.set(Pos::new(x, y), cell);
            }
        }
    }
}
