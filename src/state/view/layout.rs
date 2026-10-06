use crate::{
    state::ViewStoreTypes::TabWidth,
    types::{Face, Rgb},
};

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

#[derive(Clone, PartialEq, Eq)]
pub struct Replacements {
    pub space: Option<(char, Face)>,
    pub tab: Option<((char, char), Face)>,
    pub newline: Option<(char, Face)>,
}

impl Replacements {
    pub fn none() -> Self { Self { space: None, tab: None, newline: None } }
}

impl Default for Replacements {
    fn default() -> Self {
        Self {
            space: Some(('·', Face { fg: Some(Rgb::new(68, 71, 79)), ..Face::default() })),
            tab: Some((('›', '—'), Face { fg: Some(Rgb::new(68, 71, 79)), ..Face::default() })),
            newline: Some(('¬', Face { fg: Some(Rgb::new(68, 71, 79)), ..Face::default() })),
        }
    }
}
