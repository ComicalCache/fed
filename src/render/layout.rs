use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    render::Cell,
    state::{
        DocumentStoreTypes::Decorations as DocDecorations,
        ViewStoreTypes::Decorations as ViewDecorations,
    },
    types::{Decoration, Face},
};

#[derive(Clone)]
pub struct LayoutCell {
    pub ch: String,
    pub width: usize,
    pub face: Face,
}

pub struct Layout {
    pub cells: Vec<LayoutCell>,
    pub cursor_stops: Vec<usize>,
}

pub fn layout(
    line: &str, mut offset: usize, tab_width: usize, doc_decs: Option<&DocDecorations>,
    view_decs: Option<&ViewDecorations>,
) -> (Layout, usize) {
    let mut cells = Vec::new();
    let mut cursor_stops = Vec::new();

    let mut visual_x = 0;
    for ch in line.graphemes(true) {
        let ch_len = ch.len();
        let mut face = Face::default();
        let mut replace = false;
        let mut replacement = None;
        let mut virtual_texts = Vec::new();

        let doc_decs =
            doc_decs.map(|decs| decs.tree.find(offset, offset + ch_len)).into_iter().flatten();
        let view_decs =
            view_decs.map(|decs| decs.tree.find(offset, offset + ch_len)).into_iter().flatten();

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
            cursor_stops.push(visual_x);
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
            tab_width - (visual_x % tab_width)
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
        cursor_stops.push(visual_x);
    }
    cursor_stops.dedup();

    (Layout { cells, cursor_stops }, offset)
}
