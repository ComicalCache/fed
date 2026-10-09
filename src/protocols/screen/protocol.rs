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
            ScreenCmd::Render => Self::render(pstate),
        }
    }

    pub fn render(pstate: &mut PState) { pstate.screen.render = true }

    pub fn do_render(state: &mut State, pstate: &mut PState) {
        if !pstate.screen.render {
            return;
        }

        state.workspace.render(&state, &mut pstate.screen.screen);

        pstate.screen.screen.render();
        pstate.screen.render = false;
    }
}
