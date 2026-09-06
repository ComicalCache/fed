use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    state::{
        DocumentStoreTypes::Decorations as DocDecorations,
        ViewStoreTypes::{Decorations as ViewDecorations, TabWidth},
    },
    types::{Decoration, Face},
};

pub struct LayoutCell {
    pub ch: String,
    pub width: usize,
    pub face: Face,
}

pub struct VisualOffsetMapping {
    pub visual_x: usize,
    pub offset: usize,
}

pub struct Layout {
    pub cells: Vec<LayoutCell>,
    pub visual_cursor_stops: Vec<usize>,
    pub visual_offset_mapping: Vec<VisualOffsetMapping>,
}

pub fn layout(
    line: &str, mut offset: usize, tab_width: TabWidth, doc_decs: &DocDecorations,
    view_decs: &ViewDecorations,
) -> (Layout, usize) {
    let mut cells = Vec::new();
    let mut visual_cursor_stops = Vec::new();
    let mut visual_offset_mapping = Vec::new();

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        let ch_len = ch.len();
        let mut face = Face::default();
        let mut replace = false;
        let mut replacement = None;
        let mut virtual_texts = Vec::new();

        let doc_decs = doc_decs.tree.find(offset, offset + ch_len).into_iter();
        let view_decs = view_decs.tree.find(offset, offset + ch_len).into_iter();

        for interval in doc_decs.chain(view_decs) {
            match &interval.val {
                Decoration::Style(layer_face) => face.merge(*layer_face),
                Decoration::Replace { text, face } => {
                    replace = true;
                    if offset == interval.start {
                        replacement = Some((text.clone(), *face));
                    }
                }
                Decoration::VirtualText { text, face } => {
                    if offset == interval.start {
                        virtual_texts.push((text.clone(), *face));
                    }
                }
            }
        }

        // Virtual Text.
        for (text, face) in virtual_texts {
            for ch in text.graphemes(true) {
                let width = ch.width();
                if width == 0 {
                    break;
                }

                cells.push(LayoutCell { ch: ch.to_string(), face, width });

                for _ in 1..width {
                    cells.push(LayoutCell { ch: String::new(), face, width: 0 });
                }

                visual_x += width;
            }
        }

        if !replace || replacement.is_some() {
            visual_cursor_stops.push(visual_x);
            visual_offset_mapping.push(VisualOffsetMapping { visual_x, offset });
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

                cells.push(LayoutCell { ch: ch.to_string(), face, width });

                for _ in 1..width {
                    cells.push(LayoutCell { ch: String::new(), face, width: 0 });
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
                    cells.push(LayoutCell { ch: " ".to_string(), face, width: 1 });
                }
            } else {
                cells.push(LayoutCell { ch: ch.to_string(), face, width: ch_width });

                for _ in 1..ch_width {
                    cells.push(LayoutCell { ch: String::new(), face, width: 0 });
                }
            }

            visual_x += ch_width;
        }

        offset += ch_len;
    }

    if !line.ends_with('\n') {
        visual_cursor_stops.push(visual_x);
        visual_offset_mapping.push(VisualOffsetMapping { visual_x, offset });
    }
    visual_cursor_stops.dedup();

    (Layout { cells, visual_cursor_stops, visual_offset_mapping }, offset)
}
