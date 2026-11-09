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
    pub visual_width: usize,

    pub byte_start: usize,
    pub byte_end: usize,
}

impl VisualOffsetMapping {
    pub fn at(mappings: &[Self], visual_x: usize) -> Option<&Self> {
        let idx = mappings.partition_point(|vo| vo.visual_x + vo.visual_width <= visual_x);
        mappings.get(idx).filter(|vo| visual_x >= vo.visual_x)
    }
}

pub struct Layout {
    pub cells: Vec<Cell>,
    pub voms: Vec<VisualOffsetMapping>,
    pub next_byte_offset: usize,
}

impl Layout {
    /// Creates the layout for a given line. If `theme` is `None`, no `Cell`s
    /// are being created.
    pub fn new(
        line: &str, mut offset: usize, end: usize, decs: &[Span<Decoration>],
        layout: &ViewStoreTypes::Layout, theme: Option<&Theme>,
    ) -> Layout {
        let capacity = line.len().min(end);
        let mut result = Layout {
            // Don't create cells if no theme was supplied.
            cells: if theme.is_some() { Vec::with_capacity(capacity) } else { Vec::new() },
            voms: Vec::with_capacity(capacity),
            next_byte_offset: offset + line.len(),
        };

        let mut visual_x = 0;
        for ch in line.graphemes(true) {
            if visual_x >= end {
                return result;
            }

            let mut face = Face::default();
            let mut replace = false;
            let mut replacement = None;
            let mut virtual_texts = Vec::new();

            for span in decs {
                let zero_width = span.start == span.end;
                if span.start >= offset + ch.len()
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
                            replacement = Some((text.as_str(), *face, span.end));
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
                visual_x =
                    self::inject(text, visual_x, end, face, theme.map(|_| &mut result.cells));
            }

            let start_x = visual_x;
            if replace {
                if let Some((text, face, replacement_end)) = replacement {
                    visual_x =
                        self::inject(text, visual_x, end, face, theme.map(|_| &mut result.cells));
                    result.voms.push(VisualOffsetMapping {
                        visual_x: start_x,
                        visual_width: visual_x - start_x,
                        byte_start: offset,
                        byte_end: replacement_end,
                    });
                }
            } else {
                let width = match ch {
                    "\t" => *layout.tab_width - (visual_x % *layout.tab_width),
                    "\n" => layout
                        .replacements
                        .newline
                        .map_or(0, |(ch, _)| ch.width().unwrap_or_default()),
                    _ => ch.width(),
                };

                result.voms.push(VisualOffsetMapping {
                    visual_x,
                    visual_width: if ch == "\n" { width.max(1) } else { width },
                    byte_start: offset,
                    byte_end: if ch == "\n" && width == 0 { offset } else { offset + ch.len() },
                });

                if let Some(theme) = theme
                    && visual_x < end
                    && width > 0
                {
                    self::emit(&mut result.cells, ch, width, end - visual_x, face, layout, theme);
                }

                visual_x += width;
            }

            offset += ch.len();
        }

        if !line.ends_with('\n') {
            for span in decs {
                if span.start == offset
                    && let Decoration::VirtualText { text, face } = &span.data
                {
                    visual_x =
                        self::inject(text, visual_x, end, *face, theme.map(|_| &mut result.cells));
                }
            }

            result.voms.push(VisualOffsetMapping {
                visual_x,
                visual_width: 1,
                byte_start: offset,
                byte_end: offset,
            });
        }

        result
    }
}

fn inject(
    text: &str, mut visual_x: usize, end: usize, face: Face, mut cells: Option<&mut Vec<Cell>>,
) -> usize {
    for ch in text.graphemes(true) {
        let width = ch.width();
        if width == 0 {
            continue;
        }

        if visual_x < end
            && let Some(cells) = cells.as_deref_mut()
        {
            cells.push(Cell::new(ch.to_string(), width, face));
            for _ in 1..width {
                cells.push(Cell::new(String::new(), 0, face));
            }
        }

        visual_x += width;
    }

    visual_x
}

fn emit(
    cells: &mut Vec<Cell>, ch: &str, width: usize, available_width: usize, mut face: Face,
    layout: &ViewStoreTypes::Layout, theme: &Theme,
) {
    let replacement = match ch {
        "\t" => layout.replacements.tab.map(|((start, fill), face)| (start, Some(fill), face)),
        "\n" => layout.replacements.newline.map(|(ch, face)| (ch, None, face)),
        " " => layout.replacements.space.map(|(ch, face)| (ch, None, face)),
        _ => None,
    };

    if let Some((start, fill, replacement_face)) = replacement {
        face.merge(theme.whitespace);
        face.merge(replacement_face);

        cells.push(Cell::new(start.to_string(), start.width().unwrap_or_default(), face));
        if let Some(fill) = fill {
            for _ in 1..width.min(available_width) {
                cells.push(Cell::new(fill.to_string(), fill.width().unwrap_or_default(), face));
            }
        }
    } else if ch == "\t" {
        for _ in 0..width.min(available_width) {
            cells.push(Cell::new(" ".to_string(), 1, face));
        }
    } else {
        cells.push(Cell::new(ch.to_string(), width, face));
        for _ in 1..width {
            cells.push(Cell::new(String::new(), 0, face));
        }
    }
}
