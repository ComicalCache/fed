use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    render::Cell,
    state::ViewStoreTypes,
    types::{Decoration, Face, Span},
};

pub struct VisualOffsetMapping {
    pub visual_x: usize,
    pub offset: usize,
    pub width: usize,
}

pub fn layout_cells(
    line: &str, mut offset: usize, decs: &[Span<Decoration>],
    replacements: &ViewStoreTypes::Replacements, tab_width: ViewStoreTypes::TabWidth,
) -> Vec<Cell> {
    let mut cells = Vec::new();

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        let ch_len = ch.len();
        let mut face = Face::default();
        let mut replace = false;
        let mut replacement = None;
        let mut virtual_texts = Vec::new();

        for span in decs {
            let zero_width = span.start == span.end;
            if (zero_width && (span.start < offset || span.start >= offset + ch_len))
                || (!zero_width && (span.start >= offset + ch_len || span.end <= offset))
            {
                continue;
            }

            match &span.data {
                Decoration::Style { face: layer_face } => face.merge(*layer_face),
                Decoration::Replace { text, face } => {
                    replace = true;
                    if offset == span.start {
                        replacement = Some((text.clone(), *face));
                    }
                }
                Decoration::VirtualText { text, face } => {
                    if offset == span.start {
                        virtual_texts.push((text.clone(), *face));
                    }
                }
            }
        }

        for (text, face) in virtual_texts {
            for ch in text.graphemes(true) {
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

        if replace {
            let Some((text, face)) = replacement else {
                offset += ch_len;
                continue;
            };

            for ch in text.graphemes(true) {
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
            *tab_width - (visual_x % *tab_width)
        } else if ch == "\n" {
            if let Some((newline, _)) = replacements.newline {
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
                if let Some(((start, fill), tab_face)) = replacements.tab {
                    let mut face = face;
                    face.merge(tab_face);

                    cells.push(Cell::new(
                        start.to_string(),
                        start.width().unwrap_or_default(),
                        face,
                    ));
                    for _ in 1..ch_width {
                        cells.push(Cell::new(
                            fill.to_string(),
                            fill.width().unwrap_or_default(),
                            face,
                        ));
                    }
                } else {
                    for _ in 0..ch_width {
                        cells.push(Cell::new(" ".to_string(), 1, face));
                    }
                }
            } else if ch == "\n" {
                if let Some((newline, newline_face)) = replacements.newline {
                    let mut face = face;
                    face.merge(newline_face);

                    cells.push(Cell::new(
                        newline.to_string(),
                        newline.width().unwrap_or_default(),
                        face,
                    ));
                }
            } else if ch == " " {
                if let Some((space, space_face)) = replacements.space {
                    let mut face = face;
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
                let width = ch.width();
                if width == 0 {
                    break;
                }

                cells.push(Cell::new(ch.to_string(), width, face));

                for _ in 1..width {
                    cells.push(Cell::new(String::new(), 0, face));
                }
            }
        }
    }

    cells
}

pub fn layout_vom(
    line: &str, mut offset: usize, decs: &[Span<Decoration>],
    replacements: &ViewStoreTypes::Replacements, tab_width: ViewStoreTypes::TabWidth,
) -> (Vec<VisualOffsetMapping>, usize) {
    let mut vom = Vec::new();

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        let ch_len = ch.len();
        let mut replace = false;
        let mut replacement_text = None;
        let mut virtual_texts = Vec::new();

        for span in decs {
            let zero_width = span.start == span.end;
            if (zero_width && (span.start < offset || span.start >= offset + ch_len))
                || (!zero_width && (span.start >= offset + ch_len || span.end <= offset))
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
                ch_width = text.graphemes(true).map(|ch| ch.width()).sum();
                map_width = ch_width;
            }
        } else if ch == "\t" {
            ch_width = *tab_width - (visual_x % *tab_width);
            map_width = ch_width;
        } else if ch == "\n" {
            if let Some((newline, _)) = replacements.newline {
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
