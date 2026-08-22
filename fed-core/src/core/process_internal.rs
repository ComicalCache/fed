use std::path::PathBuf;

use piece_table::PieceTable;
use tokio::sync::oneshot;

use crate::{
    Core,
    document::DocumentId,
    messages::{CoreCommandError, CoreEvent},
};

impl Core {
    pub(super) fn finalize_create(
        &mut self, path: PathBuf, content: Result<String, String>,
        reply: oneshot::Sender<Result<DocumentId, CoreCommandError>>,
    ) {
        let Ok(content) = content else {
            let _ = reply.send(Err(CoreCommandError::FileIo(content.unwrap_err())));

            return;
        };

        let id = self.create_document(Some(path), PieceTable::from(content));

        let _ = reply.send(Ok(id));

        let event = CoreEvent::Created { id };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }

    pub(super) fn finalize_save(
        &mut self, id: DocumentId, path: PathBuf, len: usize, res: Result<(), String>,
        reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    ) {
        if let Err(err) = res {
            let _ = reply.send(Err(CoreCommandError::FileIo(err.to_string())));

            return;
        }

        if let Some(doc) = self.documents.get_mut(&id) {
            doc.modified = false;
        }

        let _ = reply.send(Ok(len));

        let event = CoreEvent::Saved { id, path, len };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }
}
