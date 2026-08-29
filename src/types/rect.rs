#[derive(Clone, Copy)]
pub enum RectSplit {
    Vertical,
    Horizontal,
}

#[derive(Default, Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,

    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self { x, y, width, height }
    }

    /// Splits the `Rect` reserving `ratio` to the first split.
    pub fn split(&self, dir: RectSplit, ratio: f32) -> (Self, Self) {
        match dir {
            RectSplit::Vertical => {
                let w1 = (self.width as f32 * ratio).round() as usize;
                let w2 = self.width.saturating_sub(w1);

                (
                    Rect::new(self.x, self.y, w1, self.height),
                    Rect::new(self.x + w1, self.y, w2, self.height),
                )
            }
            RectSplit::Horizontal => {
                let h1 = (self.height as f32 * ratio).round() as usize;
                let h2 = self.height.saturating_sub(h1);

                (
                    Rect::new(self.x, self.y, self.width, h1),
                    Rect::new(self.x, self.y + h1, self.width, h2),
                )
            }
        }
    }

    /// Checks if other intersects the rect in x and y.
    pub fn intersects(&self, other: Rect) -> (bool, bool) {
        (
            self.x < other.x + other.width && self.x + self.width > other.x,
            self.y < other.y + other.height && self.y + self.height > other.y,
        )
    }

    /// Calculates the Manhattan distance to the other rectangle.
    pub fn distance(&self, other: Rect) -> usize {
        (self.x + self.width / 2).abs_diff(other.x + other.width / 2)
            + (self.y + self.height / 2).abs_diff(other.y + other.height / 2)
    }
}
