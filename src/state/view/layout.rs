use crate::types::Face;
use crate::types::Rect;
use crate::util::newtype;

newtype!(TabWidth, usize, Clone, Copy, PartialEq, Eq, Hash);

impl Default for TabWidth {
    fn default() -> Self { Self(4) }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Replacements {
    pub space: Option<(char, Face)>,
    pub tab: Option<((char, char), Face)>,
    pub newline: Option<(char, Face)>,
}

impl Replacements {
    pub const fn none() -> Self { Self { space: None, tab: None, newline: None } }
}

impl Default for Replacements {
    fn default() -> Self {
        Self {
            space: Some(('·', Face::default())),
            tab: Some((('›', '—'), Face::default())),
            newline: Some(('¬', Face::default())),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ViewLayout {
    pub gutter: Rect,
    pub mode_line: Rect,
    pub content: Rect,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Layout {
    pub tab_width: TabWidth,

    pub gutter: bool,
    // FIXME: should this also just be a flag?
    pub mode_line: usize,
    pub replacements: Replacements,
    pub rulers: Vec<usize>,
}

impl Layout {
    pub fn gutter_width(&self, doc_lines: usize) -> usize {
        if !self.gutter {
            return 0;
        }

        // Line number width plus two for padding.
        doc_lines.checked_ilog10().unwrap_or(0) as usize + 1 + 2
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            tab_width: TabWidth::default(),
            gutter: true,
            mode_line: 1,
            replacements: Replacements::default(),
            rulers: vec![100],
        }
    }
}
