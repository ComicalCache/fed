use crate::types::Pos;

#[derive(Clone, Copy)]
pub enum RectSplit {
    Vertical,
    Horizontal,
}

#[derive(Default, Clone, Copy)]
pub struct Rect {
    pub pos: Pos,

    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn new(pos: Pos, width: usize, height: usize) -> Self { Self { pos, width, height } }

    /// Splits the `Rect` reserving `ratio` to the first split.
    pub fn split(&self, dir: RectSplit, ratio: f32) -> (Self, Self) {
        match dir {
            RectSplit::Vertical => {
                let w1 = (self.width as f32 * ratio).round() as usize;
                let w2 = self.width.saturating_sub(w1);

                (
                    Rect::new(self.pos, w1, self.height),
                    Rect::new(self.pos + Pos::new(w1, 0), w2, self.height),
                )
            }
            RectSplit::Horizontal => {
                let h1 = (self.height as f32 * ratio).round() as usize;
                let h2 = self.height.saturating_sub(h1);

                (
                    Rect::new(self.pos, self.width, h1),
                    Rect::new(self.pos + Pos::new(0, h1), self.width, h2),
                )
            }
        }
    }

    pub fn contains(&self, pos: Pos) -> bool {
        pos.x >= self.pos.x
            && pos.x < self.pos.x + self.width
            && pos.y >= self.pos.y
            && pos.y < self.pos.y + self.height
    }

    /// Checks if other intersects the rect in x and y.
    pub fn intersects(&self, other: Rect) -> (bool, bool) {
        (
            self.pos.x < other.pos.x + other.width && self.pos.x + self.width > other.pos.x,
            self.pos.y < other.pos.y + other.height && self.pos.y + self.height > other.pos.y,
        )
    }

    pub fn manhattan_distance(&self, other: Rect) -> usize {
        (self.pos.x + self.width / 2).abs_diff(other.pos.x + other.width / 2)
            + (self.pos.y + self.height / 2).abs_diff(other.pos.y + other.height / 2)
    }
}
