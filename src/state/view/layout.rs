#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub gutter: usize,
    pub mode_line: usize,
}

impl Default for Layout {
    fn default() -> Self { Self { gutter: 2, mode_line: 1 } }
}
