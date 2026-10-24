use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

use crate::fed::FCmd;
use crate::protocols::mp::MpCmd;
use crate::protocols::mp::MpProtocol;
use crate::state::State;

pub fn message(s: &mut State, message: String, duration: Duration) {
    let Some(id) = MpProtocol::message(s, message, None) else { return };

    let tx = s.async_fcmd_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(duration).await;

        let _ = tx.send(FCmd::Mp(MpCmd::Close { id }));
    });
}

pub fn async_message(tx: UnboundedSender<FCmd>, message: String, duration: Duration) {
    let (id_tx, id_rx) = oneshot::channel();
    let _ = tx.send(FCmd::Mp(MpCmd::Message { message, tx: Some(id_tx) }));

    tokio::spawn(async move {
        let Ok(id) = id_rx.await else { return };

        tokio::time::sleep(duration).await;

        let _ = tx.send(FCmd::Mp(MpCmd::Close { id }));
    });
}
