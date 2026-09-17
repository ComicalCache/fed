#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub gutter: bool,
    // FIXME: should this also just be a flag?
    pub mode_line: usize,
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
    fn default() -> Self { Self { gutter: true, mode_line: 1 } }
}
