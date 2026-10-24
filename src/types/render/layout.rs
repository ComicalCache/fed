use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::state::ViewStoreTypes;
use crate::types::Decoration;
use crate::types::Face;
use crate::types::Span;
use crate::types::Theme;
use crate::types::render::Cell;

pub struct VisualOffsetMapping {
    pub visual_x: usize,
    pub offset: usize,
    pub width: usize,
}

impl VisualOffsetMapping {
    pub fn at(mappings: &[Self], visual_x: usize) -> Option<&Self> {
        let idx = mappings.partition_point(|vo| vo.visual_x + vo.width <= visual_x);
        mappings.get(idx).filter(|vo| visual_x >= vo.visual_x)
    }
}

pub fn layout_cells(
    line: &str, mut offset: usize, end: usize, decs: &[Span<Decoration>],
    layout: &ViewStoreTypes::Layout, theme: &Theme,
) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(line.len().min(end));

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        if visual_x >= end {
            return cells;
        }

        let ch_len = ch.len();
        let mut face = Face::default();
        let mut replace = false;
        let mut replacement = None;
        let mut virtual_texts = Vec::new();

        for span in decs {
            let zero_width = span.start == span.end;
            if span.start >= offset + ch_len
                || zero_width && span.start < offset
                || !zero_width && span.end <= offset
            {
                continue;
            }

            match &span.data {
                Decoration::Style { face: layer_face } => face.merge(*layer_face),
                Decoration::Replace { text, face } => {
                    replace = true;
                    if offset == span.start {
                        replacement = Some((text.as_str(), *face));
                    }
                }
                Decoration::VirtualText { text, face } => {
                    if offset == span.start {
                        virtual_texts.push((text.as_str(), *face));
                    }
                }
            }
        }

        for (text, face) in virtual_texts {
            for ch in text.graphemes(true) {
                if visual_x >= end {
                    return cells;
                }

                let width = ch.width();
                if width == 0 {
                    break;
                }

                cells.push(Cell::new(ch.to_string(), width, face));

                for _ in 1..width {
                    cells.push(Cell::new(String::new(), 0, face));
                }

                visual_x += width;
            }
        }

        if visual_x >= end {
            return cells;
        }

        if replace {
            let Some((text, face)) = replacement else {
                offset += ch_len;
                continue;
            };

            for ch in text.graphemes(true) {
                if visual_x >= end {
                    return cells;
                }

                let width = ch.width();
                if width == 0 {
                    break;
                }

                cells.push(Cell::new(ch.to_string(), width, face));

                for _ in 1..width {
                    cells.push(Cell::new(String::new(), 0, face));
                }

                visual_x += width;
            }

            offset += ch_len;
            continue;
        }

        let ch_width = if ch == "\t" {
            *layout.tab_width - (visual_x % *layout.tab_width)
        } else if ch == "\n" {
            if let Some((newline, _)) = layout.replacements.newline {
                newline.width().unwrap_or_default()
            } else {
                // "\n".width() == 1!
                0
            }
        } else {
            ch.width()
        };

        if ch_width > 0 {
            if ch == "\t" {
                if let Some(((start, fill), tab_face)) = layout.replacements.tab {
                    let mut face = face;
                    face.merge(theme.whitespace);
                    face.merge(tab_face);

                    cells.push(Cell::new(
                        start.to_string(),
                        start.width().unwrap_or_default(),
                        face,
                    ));
                    for _ in 1..ch_width.min(end - visual_x) {
                        cells.push(Cell::new(
                            fill.to_string(),
                            fill.width().unwrap_or_default(),
                            face,
                        ));
                    }
                } else {
                    for _ in 0..ch_width.min(end - visual_x) {
                        cells.push(Cell::new(" ".to_string(), 1, face));
                    }
                }
            } else if ch == "\n" {
                if let Some((newline, newline_face)) = layout.replacements.newline {
                    let mut face = face;
                    face.merge(theme.whitespace);
                    face.merge(newline_face);

                    cells.push(Cell::new(
                        newline.to_string(),
                        newline.width().unwrap_or_default(),
                        face,
                    ));
                }
            } else if ch == " " {
                if let Some((space, space_face)) = layout.replacements.space {
                    let mut face = face;
                    face.merge(theme.whitespace);
                    face.merge(space_face);

                    cells.push(Cell::new(
                        space.to_string(),
                        space.width().unwrap_or_default(),
                        face,
                    ));
                } else {
                    cells.push(Cell::new(" ".to_string(), 1, face));
                }
            } else {
                cells.push(Cell::new(ch.to_string(), ch_width, face));
                for _ in 1..ch_width {
                    cells.push(Cell::new(String::new(), 0, face));
                }
            }

            visual_x += ch_width;
        }

        offset += ch_len;
    }

    if !line.ends_with('\n') {
        // EOF edge case for empty documents.
        let mut virtual_texts = Vec::new();
        for span in decs {
            if span.start != offset {
                continue;
            }

            if let Decoration::VirtualText { text, face } = &span.data {
                virtual_texts.push((text.clone(), face));
            }
        }

        for (text, &face) in virtual_texts {
            for ch in text.graphemes(true) {
                if visual_x >= end {
                    return cells;
                }

                let width = ch.width();
                if width == 0 {
                    break;
                }

                cells.push(Cell::new(ch.to_string(), width, face));

                for _ in 1..width {
                    cells.push(Cell::new(String::new(), 0, face));
                }

                visual_x += width;
            }
        }
    }

    cells
}

pub fn layout_vom(
    line: &str, mut offset: usize, end: usize, decs: &[Span<Decoration>],
    layout: &ViewStoreTypes::Layout,
) -> (Vec<VisualOffsetMapping>, usize) {
    let max_offset = offset + line.len();

    let mut vom = Vec::with_capacity(line.len().min(end));

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        if visual_x >= end {
            return (vom, max_offset);
        }

        let ch_len = ch.len();
        let mut replace = false;
        let mut replacement_text = None;
        let mut virtual_texts = Vec::new();

        for span in decs {
            let zero_width = span.start == span.end;
            if span.start >= offset + ch_len
                || zero_width && span.start < offset
                || !zero_width && span.end <= offset
            {
                continue;
            }

            match &span.data {
                Decoration::Style { .. } => {}
                Decoration::Replace { text, .. } => {
                    replace = true;

                    if offset == span.start {
                        replacement_text = Some(text);
                    }
                }
                Decoration::VirtualText { text, .. } => {
                    if offset == span.start {
                        virtual_texts.push(text);
                    }
                }
            }
        }

        for text in virtual_texts {
            for ch in text.graphemes(true) {
                visual_x += ch.width();
            }
        }

        let mut ch_width = 0;
        let mut map_width = 0;

        if replace {
            if let Some(text) = replacement_text {
                ch_width = text.graphemes(true).map(UnicodeWidthStr::width).sum();
                map_width = ch_width;
            }
        } else if ch == "\t" {
            ch_width = *layout.tab_width - (visual_x % *layout.tab_width);
            map_width = ch_width;
        } else if ch == "\n" {
            if let Some((newline, _)) = layout.replacements.newline {
                ch_width = newline.width().unwrap_or_default();
            }

            map_width = ch_width.max(1);
        } else {
            ch_width = ch.width();
            map_width = ch_width;
        }

        if !replace || replacement_text.is_some() {
            vom.push(VisualOffsetMapping { visual_x, offset, width: map_width });
        }

        visual_x += ch_width;
        offset += ch_len;
    }

    if !line.ends_with('\n') {
        // EOF edge case for empty documents.
        let mut virtual_texts = Vec::new();
        for span in decs {
            if span.start != offset {
                continue;
            }

            if let Decoration::VirtualText { text, .. } = &span.data {
                virtual_texts.push(text);
            }
        }

        for text in virtual_texts {
            for ch in text.graphemes(true) {
                visual_x += ch.width();
            }
        }

        vom.push(VisualOffsetMapping { visual_x, offset, width: 1 });
    }

    (vom, offset)
}
