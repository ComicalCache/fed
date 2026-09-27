#[derive(Default, Clone, Copy)]
pub struct Cursor {
    pub offset: usize,
    pub anchor: usize,
    pub pref_x: usize,
}

impl Cursor {
    pub fn new(offset: usize, pref_x: usize) -> Self { Self { offset, anchor: offset, pref_x } }
}
