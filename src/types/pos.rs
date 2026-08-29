#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub fn new(x: usize, y: usize) -> Self { Self { x, y } }
}
