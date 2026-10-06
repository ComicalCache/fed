use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    debug_panic::debug_panic,
    layers::{CursorLayer, DirLayer, SelectionLayer},
    protocols::view::ViewRenderer,
    render::{Cell, Renderer, Viewport, WindowId},
    state::{State, ViewId},
    types::{Pos, Rect},
};

pub struct DirRenderer {
    view: ViewId,
    inner: ViewRenderer,
}

impl DirRenderer {
    pub fn new(view: ViewId) -> Self {
        Self {
            view,
            inner: ViewRenderer::new(
                view,
                vec![Box::new(DirLayer {}), Box::new(SelectionLayer {}), Box::new(CursorLayer {})],
            ),
        }
    }
}

impl Renderer for DirRenderer {
    fn render(&self, state: &State, viewport: &mut Viewport, window: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, self.view)
        else {
            debug_panic!();
            return;
        };

        let lines = dse.doc.data.lines();

        let buffer_height = height.saturating_sub(vse.layout.mode_line);
        let buffer_width = width.saturating_sub(vse.layout.gutter_width(lines));

        let mut gutter = viewport.sub_view(Rect::new(
            Pos::new(0, 0),
            vse.layout.gutter_width(lines),
            buffer_height,
        ));
        self.render_gutter(state, &mut gutter, vse.layout.gutter_width(lines));

        let mut mode_line =
            viewport.sub_view(Rect::new(Pos::new(0, buffer_height), width, vse.layout.mode_line));
        self.render_mode_line(state, &mut mode_line);

        let mut view = viewport.sub_view(Rect::new(
            Pos::new(vse.layout.gutter_width(lines), 0),
            buffer_width,
            buffer_height,
        ));
        self.inner.render(state, &mut view, window);
    }
}

impl DirRenderer {
    fn render_gutter(&self, state: &State, viewport: &mut Viewport, width: usize) {
        // The render function checks for existance.
        let (vse, dse) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, self.view)
                .unwrap();

        let lines = dse.doc.data.lines();
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

                viewport.set(Pos::new(x, y), Cell::new(ch.to_string(), 1, state.theme.gutter));
            }
        }
    }

    fn render_mode_line(&self, state: &State, viewport: &mut Viewport) {
        // The render function checks for existance.
        let (vse, dse) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, self.view)
                .unwrap();

        // Force left padding.
        let mut left = Vec::new();
        for widget in &vse.mode_line_config.left {
            left.push(widget.render(state, vse, dse));

            // Add padding.
            left.push((" ".to_string(), state.theme.mode_line));
        }
        // Removing trailing padding.
        left.pop();

        let mut right = Vec::new();
        for widget in &vse.mode_line_config.right {
            right.push(widget.render(state, vse, dse));

            // Add padding. Keep the trailing padding as right padding.
            right.push((" ".to_string(), state.theme.mode_line));
        }

        let mut x = 0;

        for (text, span_face) in left {
            let mut face = state.theme.mode_line;
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
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), 1, state.theme.mode_line));
            x += 1;
        }

        for (text, span_face) in right {
            let mut face = state.theme.mode_line;
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
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), 1, state.theme.mode_line));
            x += 1;
        }
    }
}
