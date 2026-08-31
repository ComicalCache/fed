use crate::types::Pos;

#[derive(Default, Clone, Copy)]
pub struct Cursor {
    pub pos: Pos,
    pub pref_x: usize,
}

impl Cursor {
    pub fn new(pos: Pos, pref_x: usize) -> Self { Self { pos, pref_x } }
}
