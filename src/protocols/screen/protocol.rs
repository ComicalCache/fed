use crate::protocols::screen::ScreenCmd;
use crate::state::State;
use crate::types::render::Screen;

pub struct ScreenProtocol {}

impl ScreenProtocol {
    pub fn exec(s: &mut State, screen: &mut Screen, cmd: ScreenCmd) {
        match cmd {
            ScreenCmd::Resize(width, height) => {
                s.workspace.resize(width, height);
                screen.resize(width, height);
            }
        }
    }

    pub fn render(s: &State, screen: &mut Screen) {
        screen.begin_frame(s.theme.default);
        s.workspace.render(s, screen);
        screen.render();
    }
}
