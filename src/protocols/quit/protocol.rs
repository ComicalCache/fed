use std::time::Duration;

use crate::{protocols::pstate::PState, state::State, util};

pub struct QuitProtocol {}

impl QuitProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState) {
        pstate.quit.quit = true;

        for callback in pstate.quit.callbacks {
            match callback(state, pstate) {
                Ok(()) => continue,
                Err(message) => {
                    pstate.quit.quit = false;

                    util::mp::message(state, pstate, message, Duration::from_secs(3));

                    break;
                }
            }
        }
    }

    pub fn can_quit(pstate: &PState) -> bool { pstate.quit.quit }
}
