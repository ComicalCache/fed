use std::{ops::Bound, path::PathBuf};

use piece_table::{PieceTable, Slice};
use tokio::sync::oneshot;

use crate::{
    Core,
    core::CoreEvent,
    document::DocumentId,
    messages::{CoreCommandError, CoreCommandFacade::*, InternalCoreCommand::*},
};

impl Core {
    pub(super) fn create(
        &mut self, path: Option<PathBuf>,
        reply: oneshot::Sender<Result<DocumentId, CoreCommandError>>,
    ) {
        if let Some(path) = path {
            let tx = self.tx.clone();

            tokio::spawn(async move {
                // TODO: chunked reading.
                let content = tokio::fs::read_to_string(&path).await.map_err(|err| err.to_string());

                let _ = tx.send_async(Internal(FinalizeCreate { path, content, reply })).await;
            });

            return;
        }

        let id = self.create_document(path, PieceTable::from(""));

        let _ = reply.send(Ok(id));

        let event = CoreEvent::Created { id };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }

    pub(super) fn destroy(
        &mut self, id: DocumentId, force: bool,
        reply: oneshot::Sender<Result<(), CoreCommandError>>,
    ) {
        if let Some(doc) = self.documents.get(&id)
            && doc.modified
            && !force
        {
            let _ = reply.send(Err(CoreCommandError::UnsavedChanges));

            return;
        }

        self.documents.remove(&id);

        let _ = reply.send(Ok(()));

        let event = CoreEvent::Destroyed { id };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }

    pub(super) fn get_path(
        &self, id: DocumentId, reply: oneshot::Sender<Result<Option<PathBuf>, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let _ = reply.send(Ok(doc.path.clone()));
    }

    pub(super) fn set_path(
        &mut self, id: DocumentId, path: Option<PathBuf>,
        reply: oneshot::Sender<Result<(), CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get_mut(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        doc.path = path;
        doc.modified = true;

        let _ = reply.send(Ok(()));
    }

    pub(super) fn start_commit(&mut self, id: DocumentId) {
        if let Some(doc) = self.documents.get_mut(&id) {
            doc.data.start_commit();
        }
    }

    pub(super) fn end_commit(&mut self, id: DocumentId) {
        if let Some(doc) = self.documents.get_mut(&id) {
            doc.data.end_commit();
        }
    }

    pub(super) fn undo(&mut self, id: DocumentId) {
        if let Some(doc) = self.documents.get_mut(&id) {
            doc.data.undo();
        }
    }

    pub(super) fn hot_redo(&mut self, id: DocumentId) {
        if let Some(doc) = self.documents.get_mut(&id) {
            doc.data.hot_redo();
        }
    }

    pub(super) fn insert(&mut self, id: DocumentId, pos: usize, str: String) {
        if str.is_empty() {
            return;
        }

        let Some(doc) = self.documents.get_mut(&id) else {
            return;
        };

        let pos = pos.min(doc.data.len());

        doc.data.insert(pos, &str);
        doc.modified = true;

        let event = CoreEvent::Inserted { id, pos, str };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }

    pub(super) fn append(&mut self, id: DocumentId, str: String) {
        if str.is_empty() {
            return;
        }

        let Some(doc) = self.documents.get_mut(&id) else {
            return;
        };

        let pos = doc.data.len();

        doc.data.append(&str);
        doc.modified = true;

        let event = CoreEvent::Inserted { id, pos, str };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }

    pub(super) fn remove(&mut self, id: DocumentId, pos: usize, n: usize) {
        let Some(doc) = self.documents.get_mut(&id) else {
            return;
        };

        let pos = pos.min(doc.data.len());
        let n = n.min(doc.data.len() - pos);
        if n == 0 {
            return;
        }

        let str = doc.data.slice(pos..pos + n);

        doc.data.remove(pos, n);
        doc.modified = true;

        let event = CoreEvent::Removed { id, pos, n, str };
        for mapper in &self.event_mappers {
            mapper.map(&event);
        }
    }

    pub(super) fn get_line_start_byte(
        &self, id: DocumentId, n: usize, reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let _ = reply.send(Ok(doc.data.get_line_start_byte(n)));
    }

    pub(super) fn get_line_end_byte(
        &self, id: DocumentId, n: usize, reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let _ = reply.send(Ok(doc.data.get_line_end_byte(n)));
    }

    pub(super) fn is_modified(
        &self, id: DocumentId, reply: oneshot::Sender<Result<bool, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let _ = reply.send(Ok(doc.modified));
    }

    pub(super) fn save(
        &mut self, id: DocumentId, reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let Some(path) = doc.path.clone() else {
            let _ = reply.send(Err(CoreCommandError::NoPath));

            return;
        };

        let tx = self.tx.clone();

        let len = doc.data.len();
        let data = doc.data.to_string();
        tokio::spawn(async move {
            let res = tokio::fs::write(&path, data).await.map_err(|err| err.to_string());

            let _ = tx.send_async(Internal(FinalizeSave { id, path, len, res, reply })).await;
        });
    }

    pub(super) fn get_slice(
        &self, id: DocumentId, range: (Bound<usize>, Bound<usize>),
        reply: oneshot::Sender<Result<String, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let start = match range.0 {
            Bound::Included(n) => n,
            Bound::Excluded(n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.1 {
            Bound::Included(n) => n + 1,
            Bound::Excluded(n) => n,
            Bound::Unbounded => doc.data.len(),
        };

        let start = start.min(doc.data.len());
        let end = end.min(doc.data.len()).max(start);

        if start == end {
            let _ = reply.send(Ok(String::new()));

            return;
        }

        let _ = reply.send(Ok(doc.data.slice(start..end)));
    }

    pub(super) fn lines(
        &self, id: DocumentId, reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let _ = reply.send(Ok(doc.data.lines()));
    }

    pub(super) fn len(
        &self, id: DocumentId, reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    ) {
        let Some(doc) = self.documents.get(&id) else {
            let _ = reply.send(Err(CoreCommandError::NoDocument));

            return;
        };

        let _ = reply.send(Ok(doc.data.len()));
    }
}
