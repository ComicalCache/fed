use crossterm::style::Color;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
}

impl Into<Color> for Rgb {
    fn into(self) -> Color { Color::Rgb { r: self.r, g: self.g, b: self.b } }
}
