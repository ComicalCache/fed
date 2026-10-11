use std::time::Duration;

use crate::{protocols::state::PState, state::State, util};

pub struct QuitProtocol {}

impl QuitProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState) {
        let mut quit = true;

        for callback in pstate.quit.callbacks {
            match callback(state, pstate) {
                Ok(()) => continue,
                Err(message) => {
                    quit = false;

                    util::mp::message(state, pstate, message, Duration::from_secs(3));

                    break;
                }
            }
        }

        pstate.quit.quit = quit;
    }

    pub fn can_quit(pstate: &PState) -> bool { pstate.quit.quit }
}
