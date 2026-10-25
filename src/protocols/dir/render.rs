use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::layers::CurrLineLayer;
use crate::layers::CursorLayer;
use crate::layers::DirLayer;
use crate::layers::SelectionLayer;
use crate::protocols::view::ViewRenderer;
use crate::state::State;
use crate::types::Pos;
use crate::types::render::Cell;
use crate::types::render::Renderer;
use crate::types::render::Viewport;
use crate::types::render::WindowId;

pub struct DirRenderer {
    inner: ViewRenderer,
}

impl DirRenderer {
    pub fn new() -> Self {
        Self {
            inner: ViewRenderer::new(vec![
                Box::new(DirLayer {}),
                Box::new(CurrLineLayer {}),
                Box::new(SelectionLayer {}),
                Box::new(CursorLayer {}),
            ]),
        }
    }
}

impl Renderer for DirRenderer {
    fn render(&self, s: &State, viewport: &mut Viewport, _: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.dir.view).unwrap();

        let layout = vse.layout(dse);

        let mut gutter = viewport.sub_view(layout.gutter);
        Self::render_gutter(s, &mut gutter, layout.gutter.width);

        let mut mode_line = viewport.sub_view(layout.mode_line);
        Self::render_mode_line(s, &mut mode_line);

        let mut content = viewport.sub_view(layout.content);
        self.inner.render(s, &mut content, s.p.dir.view);
    }
}

impl DirRenderer {
    fn render_gutter(s: &State, viewport: &mut Viewport, width: usize) {
        // The render function checks for existance.
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.dir.view).unwrap();

        let lines = dse.doc.data.line_count();
        let scroll = vse.scroll;

        for y in 0..viewport.height() {
            let num = y + scroll.y + 1;
            let num = if num <= lines {
                format!("{num:>width$} ", width = width.saturating_sub(1))
            } else {
                " ".repeat(width)
            };

            for (x, ch) in num.chars().enumerate() {
                if x >= viewport.width() {
                    break;
                }

                viewport.set(Pos::new(x, y), Cell::new(ch.to_string(), 1, s.theme.gutter));
            }
        }
    }

    fn render_mode_line(s: &State, viewport: &mut Viewport) {
        // The render function checks for existance.
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.dir.view).unwrap();

        // Force left padding.
        let mut left = Vec::new();
        for widget in &vse.mode_line_config.left {
            left.push(widget.render(s, vse, dse));

            // Add padding.
            left.push((" ".to_string(), s.theme.mode_line));
        }
        // Removing trailing padding.
        left.pop();

        let mut right = Vec::new();
        for widget in &vse.mode_line_config.right {
            right.push(widget.render(s, vse, dse));

            // Add padding. Keep the trailing padding as right padding.
            right.push((" ".to_string(), s.theme.mode_line));
        }

        let mut x = 0;

        for (text, span_face) in left {
            let mut face = s.theme.mode_line;
            face.merge(span_face);

            for grapheme in text.graphemes(true) {
                let width = grapheme.width();
                if x >= viewport.width() {
                    break;
                }

                viewport.set(Pos::new(x, 0), Cell::new(grapheme.to_string(), width, face));

                for _ in 1..width {
                    if x + 1 < viewport.width() {
                        viewport.set(Pos::new(x + 1, 0), Cell::new(String::new(), 0, face));
                    }

                    x += 1;
                }

                if width > 0 {
                    x += 1;
                }
            }
        }

        let right_width = right
            .iter()
            .flat_map(|(text, _)| text.graphemes(true))
            .map(UnicodeWidthStr::width)
            .sum::<usize>();
        let right_start = viewport.width().saturating_sub(right_width);

        while x < right_start {
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), 1, s.theme.mode_line));
            x += 1;
        }

        for (text, span_face) in right {
            let mut face = s.theme.mode_line;
            face.merge(span_face);

            for grapheme in text.graphemes(true) {
                let width = grapheme.width();
                if x >= viewport.width() {
                    break;
                }

                viewport.set(Pos::new(x, 0), Cell::new(grapheme.to_string(), width, face));

                for _ in 1..width {
                    if x + 1 < viewport.width() {
                        viewport.set(Pos::new(x + 1, 0), Cell::new(String::new(), 0, face));
                    }

                    x += 1;
                }

                if width > 0 {
                    x += 1;
                }
            }
        }

        while x < viewport.width() {
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), 1, s.theme.mode_line));
            x += 1;
        }
    }
}
