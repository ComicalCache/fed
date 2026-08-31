use crate::{
    render::{Cell, Screen},
    types::{Pos, Rect},
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

    pub fn get(&mut self, pos: Pos) -> Option<&Cell> {
        if pos.x >= self.rect.width || pos.y >= self.rect.height {
            return None;
        }

        self.data.get(self.rect.pos + pos)
    }

    pub fn set(&mut self, pos: Pos, cell: Cell) {
        if pos.x >= self.rect.width || pos.y >= self.rect.height {
            return;
        }

        self.data.set(self.rect.pos + pos, cell);
    }
}
