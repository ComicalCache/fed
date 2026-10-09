use std::time::Duration;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    protocols::{
        mp::{MpCmd, MpProtocol},
        state::PState,
    },
    state::State,
};

pub struct QuitProtocol {}

impl QuitProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState) {
        let mut quit = true;

        for callback in pstate.quit.callbacks {
            match callback(state, pstate) {
                Ok(()) => continue,
                Err(message) => {
                    quit = false;

                    let Some(id) = MpProtocol::message(state, pstate, message, None) else {
                        debug_panic!();

                        pstate.quit.quit = false;

                        return;
                    };

                    let async_fcmd_tx = state.async_fcmd_tx.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(3)).await;

                        let _ = async_fcmd_tx.send(FCmd::Mp(MpCmd::Close { id }));
                    });

                    break;
                }
            }
        }

        pstate.quit.quit = quit;
    }

    pub fn can_quit(pstate: &PState) -> bool { pstate.quit.quit }
}
