use crate::state::State;
use crate::state::ViewId;
use crate::types::Pos;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::Viewport;
use crate::types::render::{self};

pub struct ViewRenderer {
    layers: Vec<Box<dyn Layer>>,
}

impl ViewRenderer {
    pub fn new(layers: Vec<Box<dyn Layer>>) -> Self { Self { layers } }
}

impl ViewRenderer {
    pub fn render(&self, s: &State, viewport: &mut Viewport, view: ViewId) {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let content = vse.layout(dse).content;
        let width = content.width.min(viewport.width());
        let height = content.height.min(viewport.height());
        if width == 0 || height == 0 {
            return;
        }

        let start = vse.view_cache.offset;
        let end = start + vse.view_cache.lines.iter().take(height).map(String::len).sum::<usize>();

        let mut decs = Vec::new();
        dse.decs.range(start, end, &mut decs);
        vse.decs.range(start, end, &mut decs);

        let mut lines_drawn = 0;
        let mut offset = vse.view_cache.offset;
        let end = vse.scroll.x.saturating_add(width);
        for (y, line) in vse.view_cache.lines.iter().enumerate() {
            if y >= height {
                break;
            }

            let (vom, next_offset) = render::layout_vom(line, offset, end, &decs, &vse.layout);
            let cells = render::layout_cells(line, offset, end, &decs, &vse.layout, &s.theme);
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
