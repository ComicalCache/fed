use crossterm::style::Color;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
}

impl From<Rgb> for Color {
    fn from(val: Rgb) -> Self { Self::Rgb { r: val.r, g: val.g, b: val.b } }
}
