use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    protocols::view::ViewRenderer,
    render::{Cell, Renderer, Viewport, WindowId},
    state::{StateLock, ViewId},
    types::{Face, Pos, Rect},
};

pub struct ViewDecoratorRenderer {
    view: ViewId,
    inner: ViewRenderer,

    state_lock: StateLock,
}

impl ViewDecoratorRenderer {
    pub fn new(view: ViewId, inner: ViewRenderer, state_lock: StateLock) -> Self {
        Self { view, inner, state_lock }
    }
}

impl Renderer for ViewDecoratorRenderer {
    fn render(&self, viewport: &mut Viewport, window: WindowId) {
        let width = viewport.width();
        let height = viewport.height();

        if height == 0 || width == 0 {
            return;
        }

        let state = self.state_lock.read();

        let Some(vse) = state.view_store.get(&self.view) else { return };
        let layout = vse.layout;

        drop(state);

        let buffer_height = height.saturating_sub(layout.mode_line);
        let buffer_width = width.saturating_sub(layout.gutter);

        let mut gutter = viewport.sub_view(Rect::new(Pos::new(0, 0), layout.gutter, buffer_height));
        self.render_gutter(&mut gutter, layout.gutter);

        let mut mode =
            viewport.sub_view(Rect::new(Pos::new(0, buffer_height), width, layout.mode_line));
        self.render_mode_line(&mut mode);

        let mut view =
            viewport.sub_view(Rect::new(Pos::new(layout.gutter, 0), buffer_width, buffer_height));
        self.inner.render(&mut view, window);
    }
}

impl ViewDecoratorRenderer {
    fn render_gutter(&self, viewport: &mut Viewport, width: usize) {
        let state = self.state_lock.read();

        let Some((vse, dse)) = state.view_and_doc(self.view) else { return };
        let lines = dse.doc.data.lines();
        let scroll = vse.scroll;

        drop(state);

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

                viewport.set(Pos::new(x, y), Cell::new(ch.to_string(), false, face));
            }
        }
    }

    fn render_mode_line(&self, viewport: &mut Viewport) {
        let state = self.state_lock.read();

        let Some((vse, dse)) = state.view_and_doc(self.view) else { return };

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

        drop(state);

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

                viewport.set(Pos::new(x, 0), Cell::new(grapheme.to_string(), width == 0, face));

                for _ in 1..width {
                    if x + 1 < viewport.width() {
                        viewport.set(Pos::new(x + 1, 0), Cell::new(String::new(), true, face));
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
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), false, base_face));
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

                viewport.set(Pos::new(x, 0), Cell::new(grapheme.to_string(), width == 0, face));

                for _ in 1..width {
                    if x + 1 < viewport.width() {
                        viewport.set(Pos::new(x + 1, 0), Cell::new(String::new(), true, face));
                    }

                    x += 1;
                }

                if width > 0 {
                    x += 1;
                }
            }
        }

        while x < viewport.width() {
            viewport.set(Pos::new(x, 0), Cell::new(" ".to_string(), false, base_face));
            x += 1;
        }
    }
}
