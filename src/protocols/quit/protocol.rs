use std::time::Duration;

use crate::state::State;
use crate::util;

pub struct QuitProtocol {}

impl QuitProtocol {
    pub fn exec(s: &mut State) {
        s.p.quit.quit = true;

        for callback in s.p.quit.callbacks {
            match callback(s) {
                Ok(()) => {}
                Err(message) => {
                    s.p.quit.quit = false;

                    util::mp::message(s, message, Duration::from_secs(3));

                    break;
                }
            }
        }
    }

    pub const fn can_quit(s: &State) -> bool { s.p.quit.quit }
}
