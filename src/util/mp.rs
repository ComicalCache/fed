// src/util/mp.rs
use std::time::Duration;

use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    protocols::{
        mp::{MpCmd, MpProtocol},
        state::PState,
    },
    state::State,
};

pub fn message(state: &mut State, pstate: &mut PState, message: String, duration: Duration) {
    let Some(id) = MpProtocol::message(state, pstate, message, None) else {
        debug_panic!();
        return;
    };

    let tx = state.async_fcmd_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(duration).await;

        let _ = tx.send(FCmd::Mp(MpCmd::Close { id }));
    });
}

pub fn async_message(tx: UnboundedSender<FCmd>, message: String, duration: Duration) {
    let (id_tx, id_rx) = oneshot::channel();
    let _ = tx.send(FCmd::Mp(MpCmd::Message { message, tx: Some(id_tx) }));

    tokio::spawn(async move {
        let Ok(id) = id_rx.await else {
            debug_panic!();
            return;
        };

        tokio::time::sleep(duration).await;

        let _ = tx.send(FCmd::Mp(MpCmd::Close { id }));
    });
}
