use crate::{
    render::{Cell, Screen},
    types::Rect,
};

/// A bounded proxy for the `Screen`, limited to a rectangle on the screen.
pub struct Viewport<'a> {
    data: &'a mut Screen,
    rect: Rect,
}

impl<'a> Viewport<'a> {
    pub fn new(data: &'a mut Screen, rect: Rect) -> Self { Self { data, rect } }

    pub fn width(&self) -> usize { self.rect.width }

    pub fn height(&self) -> usize { self.rect.height }

    pub fn get(&mut self, x: usize, y: usize) -> Option<&Cell> {
        if x >= self.rect.width || y >= self.rect.height {
            return None;
        }

        self.data.get(self.rect.x + x, self.rect.y + y)
    }

    pub fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x >= self.rect.width || y >= self.rect.height {
            return;
        }

        self.data.set(self.rect.x + x, self.rect.y + y, cell);
    }
}
