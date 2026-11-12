use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::state::State;
use crate::state::ViewId;
use crate::types::Pos;
use crate::types::render::Cell;
use crate::types::render::Viewport;

pub fn render_gutter(
    s: &State, viewport: &mut Viewport, mut lines: impl Iterator<Item = Option<usize>>,
) {
    let width = viewport.width();

    for y in 0..viewport.height() {
        let text = lines.next().flatten().map_or_else(
            || " ".repeat(width),
            |line| format!("{line:>digits$} ", digits = width.saturating_sub(1)),
        );

        for (x, ch) in text.chars().take(width).enumerate() {
            viewport.set(Pos::new(x, y), Cell::new(ch.to_string(), 1, s.theme.gutter));
        }
    }
}

pub fn render_mode_line(s: &State, viewport: &mut Viewport, view: ViewId) {
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    // Force left padding.
    let mut left = Vec::new();
    for widget in &vse.mode_line_config.left {
        let rendered = widget.render(s, vse, dse);
        if rendered.0.is_empty() {
            continue;
        }
        left.push(rendered);

        // Add padding.
        left.push((" ".to_string(), s.theme.mode_line));
    }
    // Removing trailing padding.
    left.pop();

    let mut right = Vec::new();
    for widget in &vse.mode_line_config.right {
        let rendered = widget.render(s, vse, dse);
        if rendered.0.is_empty() {
            continue;
        }
        right.push(rendered);

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
