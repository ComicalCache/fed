use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    render::Cell,
    state::ViewStoreTypes::TabWidth,
    types::{Decoration, Face, Span},
};

pub struct VisualOffsetMapping {
    pub visual_x: usize,
    pub offset: usize,
}

pub fn layout_cells(
    line: &str, mut offset: usize, tab_width: TabWidth, decs: &[Span<Decoration>],
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
            if span.start > offset + ch_len || span.end < offset {
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
            // "\n".width() == 1!
            0
        } else {
            ch.width()
        };

        if ch_width > 0 {
            if ch == "\t" {
                for _ in 0..ch_width {
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
    line: &str, mut offset: usize, tab_width: TabWidth, decs: &[Span<Decoration>],
) -> (Vec<VisualOffsetMapping>, usize) {
    let mut vom = Vec::new();

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        let ch_len = ch.len();
        let mut replace = false;
        let mut replacement_text = None;
        let mut virtual_texts = Vec::new();

        for span in decs {
            if span.start > offset + ch_len || span.end < offset {
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

        if !replace || replacement_text.is_some() {
            vom.push(VisualOffsetMapping { visual_x, offset });
        }

        if replace {
            let Some(text) = replacement_text else {
                offset += ch_len;

                continue;
            };

            for ch in text.graphemes(true) {
                visual_x += ch.width();
            }

            offset += ch_len;

            continue;
        }

        let ch_width = if ch == "\t" {
            *tab_width - (visual_x % *tab_width)
        } else if ch == "\n" {
            // "\n".width() == 1!
            0
        } else {
            ch.width()
        };

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

        vom.push(VisualOffsetMapping { visual_x, offset });
    }

    (vom, offset)
}
