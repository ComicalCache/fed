use crate::render::Screen;

pub struct ScreenState {
    pub screen: Screen,
}

impl ScreenState {
    pub fn new(width: usize, height: usize) -> Self { Self { screen: Screen::new(width, height) } }
}
