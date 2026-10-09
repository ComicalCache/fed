use crate::{
    protocols::{screen::ScreenCmd, state::PState},
    state::State,
};

pub struct ScreenProtocol {}

impl ScreenProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState, cmd: ScreenCmd) {
        match cmd {
            ScreenCmd::Resize(width, height) => {
                state.workspace.resize(width, height);
                pstate.screen.screen.resize(width, height)
            }
        }
    }

    pub fn render(state: &mut State, pstate: &mut PState) {
        state.workspace.render(&state, &mut pstate.screen.screen);
        pstate.screen.screen.render();
    }
}
