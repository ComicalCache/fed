use crate::render::Screen;

pub struct ScreenPState {
    pub screen: Screen,
}

impl ScreenPState {
    pub fn new(width: usize, height: usize) -> Self { Self { screen: Screen::new(width, height) } }
}
