use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub fn new(x: usize, y: usize) -> Self { Self { x, y } }
}

impl Add for Pos {
    type Output = Pos;

    fn add(self, rhs: Self) -> Self::Output { Pos::new(self.x + rhs.x, self.y + rhs.y) }
}

impl AddAssign for Pos {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Pos {
    type Output = Pos;

    fn sub(self, rhs: Self) -> Self::Output { Pos::new(self.x - rhs.x, self.y - rhs.y) }
}

impl SubAssign for Pos {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Pos {
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Pos::new(self.x.saturating_sub(rhs.x), self.y.saturating_sub(rhs.y))
    }
}

impl<T: Into<usize>> From<(T, T)> for Pos {
    fn from((x, y): (T, T)) -> Self { Pos::new(x.into(), y.into()) }
}
