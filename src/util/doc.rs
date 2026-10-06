use std::path::PathBuf;

use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    protocols::io::IoCommand,
    state::{DocId, StateLock},
    util,
};

pub async fn create_doc(
    state_lock: StateLock, path: Option<PathBuf>, io: UnboundedSender<IoCommand>,
) -> DocId {
    let path = path.map(|p| util::path::normalize(p));

    if let Some(path) = &path
        && let Some((&doc, _)) =
            state_lock.read().doc_store.iter().find(|(_, dse)| dse.doc.path.as_ref() == Some(path))
    {
        return doc;
    }

    let data = if let Some(path) = path.clone() {
        let (tx, rx) = oneshot::channel();
        if io.send(IoCommand::Read { path, tx }).is_err() {
            todo!("Exit with error");
        }

        rx.await.map_err(|err| err.to_string()).flatten().unwrap_or_else(|_| String::new())
    } else {
        String::new()
    };

    let doc = state_lock.write().create_doc(path, data);

    doc
}
