use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    debug_panic::debug_panic,
    protocols::view::ViewRenderer,
    render::{Cell, Renderer, Viewport, WindowId},
    state::{State, ViewId},
    types::{Face, Pos, Rect},
};

pub struct ViewDecoratorRenderer {
    view: ViewId,
    inner: ViewRenderer,
}

impl ViewDecoratorRenderer {
    pub fn new(view: ViewId, inner: ViewRenderer) -> Self { Self { view, inner } }
}

impl Renderer for ViewDecoratorRenderer {
    fn render(&self, state: &State, viewport: &mut Viewport, window: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let Some((vse, dse)) = state.vse_and_dse(self.view) else {
            debug_panic!();
            return;
        };

        let lines = dse.doc.data.lines();
        let layout = vse.layout;

        let buffer_height = height.saturating_sub(layout.mode_line);
        let buffer_width = width.saturating_sub(layout.gutter_width(lines));

        let mut gutter =
            viewport.sub_view(Rect::new(Pos::new(0, 0), layout.gutter_width(lines), buffer_height));
        self.render_gutter(state, &mut gutter, layout.gutter_width(lines));

        let mut mode_line =
            viewport.sub_view(Rect::new(Pos::new(0, buffer_height), width, layout.mode_line));
        self.render_mode_line(state, &mut mode_line);

        let mut view = viewport.sub_view(Rect::new(
            Pos::new(layout.gutter_width(lines), 0),
            buffer_width,
            buffer_height,
        ));
        self.inner.render(state, &mut view, window);
    }
}

impl ViewDecoratorRenderer {
    fn render_gutter(&self, state: &State, viewport: &mut Viewport, width: usize) {
        // The render function checks for existance.
        let (vse, dse) = state.vse_and_dse(self.view).unwrap();

        let lines = dse.doc.data.lines();
        let scroll = vse.scroll;

        let face = Face::default();
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

                viewport.set(Pos::new(x, y), Cell::new(ch.to_string(), 1, face));
            }
        }
    }

    fn render_mode_line(&self, state: &State, viewport: &mut Viewport) {
        // The render function checks for existance.
        let (vse, dse) = state.vse_and_dse(self.view).unwrap();

        // Force left padding.
        let mut left = Vec::new();
        for widget in &vse.mode_line_config.left {
            left.extend(widget.render(vse, dse));

            // Add padding.
            left.push((" ".to_string(), Face::default()));
        }
        // Removing trailing padding.
        left.pop();

        let mut right = Vec::new();
        for widget in &vse.mode_line_config.right {
            right.extend(widget.render(vse, dse));

            // Add padding. Keep the trailing padding as right padding.
            right.push((" ".to_string(), Face::default()));
        }

        let mut base_face = Face::default();
        base_face.reverse = Some(true);

        let mut x = 0;

        for (text, span_face) in left {
            let mut face = base_face;
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
            .map(|g| g.width())
            .sum::<usize>();
        let right_start = viewport.width().saturating_sub(right_width);

        while x < right_start {
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), 1, base_face));
            x += 1;
        }

        for (text, span_face) in right {
            let mut face = base_face;
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
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), 1, base_face));
            x += 1;
        }
    }
}
