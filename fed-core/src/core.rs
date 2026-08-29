mod process;
mod process_internal;

use std::{collections::HashMap, path::PathBuf};

use piece_table::PieceTable;

use crate::{
    CoreCommandSender,
    document::{Document, DocumentId},
    messages::{CoreCommand, CoreCommandFacade, CoreEvent, CoreEventMapper, InternalCoreCommand},
};

/// This is the application core, containing the `Core::run` application-loop.
pub struct Core {
    documents: HashMap<DocumentId, Document>,
    next_document_id: u64,

    event_mappers: Vec<Box<dyn CoreEventMapper>>,

    rx: flume::Receiver<CoreCommandFacade>,
    tx: flume::Sender<CoreCommandFacade>,
}

impl Core {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();

        Self { documents: HashMap::new(), next_document_id: 0, event_mappers: Vec::new(), rx, tx }
    }

    /// Returns a channel for protocols to send commands to the core.
    pub fn tx(&self) -> CoreCommandSender { CoreCommandSender { tx: self.tx.clone() } }

    /// Adds a `CoreEventMapper` to map outgoing `CoreEvent`s.
    pub fn add_event_mapper(&mut self, event_mapper: Box<dyn CoreEventMapper>) {
        self.event_mappers.push(event_mapper);
    }

    /// Runs the core's event loop, waiting for and reacting to `CoreCommand`s,
    /// emitting `CoreEvent`s in response, when applicable.
    pub async fn run(&mut self) {
        use CoreCommandFacade::*;

        while let Ok(cmd) = self.rx.recv_async().await {
            match cmd {
                Public(cmd) => self.process_command(cmd),
                Internal(cmd) => self.process_internal_command(cmd),
            }
        }
    }

    fn process_command(&mut self, cmd: CoreCommand) {
        use CoreCommand::*;

        match cmd {
            Create { path, reply } => self.create(path, reply),
            Destroy { id, force, reply } => self.destroy(id, force, reply),
            GetPath { id, reply } => self.get_path(id, reply),
            SetPath { id, path, reply } => self.set_path(id, path, reply),
            StartCommit { id } => self.start_commit(id),
            EndCommit { id } => self.end_commit(id),
            Undo { id } => self.undo(id),
            HotRedo { id } => self.hot_redo(id),
            Insert { id, pos, str } => self.insert(id, pos, str),
            Append { id, str } => self.append(id, str),
            Remove { id, pos, n } => self.remove(id, pos, n),
            GetLineStartByte { id, n, reply } => self.get_line_start_byte(id, n, reply),
            GetLineEndByte { id, n, reply } => self.get_line_end_byte(id, n, reply),
            IsModified { id, reply } => self.is_modified(id, reply),
            Save { id, reply } => self.save(id, reply),
            GetSlice { id, range, reply } => self.get_slice(id, range, reply),
            Lines { id, reply } => self.lines(id, reply),
            Len { id, reply } => self.len(id, reply),
            Batch(cmds) => {
                for cmd in cmds.into_iter().filter(|cmd| !matches!(cmd, CoreCommand::Batch(_))) {
                    self.process_command(cmd);
                }
            }
        }
    }

    fn process_internal_command(&mut self, cmd: InternalCoreCommand) {
        use InternalCoreCommand::*;

        match cmd {
            FinalizeCreate { path, content, reply } => self.finalize_create(path, content, reply),
            FinalizeSave { id, path, len, res, reply } => {
                self.finalize_save(id, path, len, res, reply)
            }
        }
    }

    fn create_document(&mut self, path: Option<PathBuf>, data: PieceTable) -> DocumentId {
        let id = DocumentId(self.next_document_id);
        self.next_document_id += 1;

        self.documents.insert(id, Document::new(path, data));

        id
    }
}
