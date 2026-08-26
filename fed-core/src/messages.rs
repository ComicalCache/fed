use std::{
    ops::{Bound, RangeBounds},
    path::PathBuf,
};

use tokio::sync::oneshot;

use crate::document::DocumentId;

/// A wrapper for protocols to send `CoreCommand`s to the core.
pub struct CoreCommandSender {
    pub(crate) tx: flume::Sender<CoreCommandFacade>,
}

impl CoreCommandSender {
    pub async fn send_async(&self, cmd: CoreCommand) -> Result<(), flume::SendError<CoreCommand>> {
        self.tx.send_async(CoreCommandFacade::Public(cmd)).await.map_err(|err| {
            let CoreCommandFacade::Public(cmd) = err.into_inner() else { unreachable!() };
            flume::SendError(cmd)
        })
    }

    /// Convenience function for `Document` creation. See
    /// `CoreCommand::CreateDocument` for more details.
    pub async fn create(&self, path: Option<PathBuf>) -> Result<DocumentId, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::Create { path, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for `Document` destruction. See
    /// `CoreCommand::DestroyDocument` for more details.
    pub async fn destroy(&self, id: DocumentId, force: bool) -> Result<(), CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::Destroy { id, force, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for getting a `Document`s path. See
    /// `CoreCommand::GetPath` for more details.
    pub async fn get_path(&self, id: DocumentId) -> Result<Option<PathBuf>, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::GetPath { id, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for setting a `Document`s path. See
    /// `CoreCommand::SetPath` for more details.
    pub async fn set_path(
        &self, id: DocumentId, path: Option<PathBuf>,
    ) -> Result<(), CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::SetPath { id, path, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for starting a commit for a `Document`. See
    /// `CoreCommand::StartCommit` for more details.
    pub async fn start_commit(&self, id: DocumentId) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::StartCommit { id };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function for starting a commit for a `Document`. See
    /// `CoreCommand::EndCommit` for more details.
    pub async fn end_commit(&self, id: DocumentId) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::EndCommit { id };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function for undoing the last commit for a `Document`. See
    /// `CoreCommand::Undo` for more details.
    pub async fn undo(&self, id: DocumentId) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::Undo { id };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function for redoing the last commit for a `Document`. See
    /// `CoreCommand::HotRedo` for more details.
    pub async fn hot_redo(&self, id: DocumentId) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::HotRedo { id };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function for inserting into a `Document`. See
    /// `CoreCommand::Insert` for more details.
    pub async fn insert(
        &self, id: DocumentId, pos: usize, str: String,
    ) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::Insert { id, pos, str };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function for appending to a `Document`. See
    /// `CoreCommand::Append` for more details.
    pub async fn append(&self, id: DocumentId, str: String) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::Append { id, str };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function for removing from a `Document`. See
    /// `CoreCommand::Remove` for more details.
    pub async fn remove(
        &self, id: DocumentId, pos: usize, n: usize,
    ) -> Result<(), CoreCommandError> {
        let cmd = CoreCommand::Remove { id, pos, n };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        Ok(())
    }

    /// Convenience function to get the byte where the nth line begins.
    pub async fn get_line_start_byte(
        &self, id: DocumentId, n: usize,
    ) -> Result<usize, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::GetLineStartByte { id, n, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function to get the byte index of where the nth line ends
    /// and the next line begins.
    pub async fn get_line_end_byte(
        &self, id: DocumentId, n: usize,
    ) -> Result<usize, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::GetLineEndByte { id, n, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function to check if a `Document` has unsaved changes. See
    /// `CoreCommand::IsModified` for more details.
    pub async fn is_modified(&self, id: DocumentId) -> Result<bool, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::IsModified { id, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for saving a `Document`. See `CoreCommand::Save`
    /// for more details.
    pub async fn save(&self, id: DocumentId) -> Result<usize, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::Save { id, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for getting a slice of a `Document`. See
    /// `CoreCommand::Slice` for more details.
    pub async fn get_slice<R: RangeBounds<usize>>(
        &self, id: DocumentId, range: R,
    ) -> Result<String, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let range = (range.start_bound().cloned(), range.end_bound().cloned());
        let cmd = CoreCommand::GetSlice { id, range, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }

    /// Convenience function for getting the length of a `Document`. See
    /// `CoreCommand::Len` for more details.
    pub async fn len(&self, id: DocumentId) -> Result<usize, CoreCommandError> {
        let (tx, rx) = oneshot::channel();

        let cmd = CoreCommand::Len { id, reply: tx };
        if self.tx.send_async(CoreCommandFacade::Public(cmd)).await.is_err() {
            return Err(CoreCommandError::Offline);
        }

        rx.await.unwrap_or_else(|_| Err(CoreCommandError::Dropped))
    }
}

/// A command for the core to execute. Commands can be sent via the
/// `CoreCommandSender`.
pub enum CoreCommand {
    /// Create a new `Document`. If the path is `None` a "scratchpad" document
    /// will be created with no underlying file.
    Create { path: Option<PathBuf>, reply: oneshot::Sender<Result<DocumentId, CoreCommandError>> },

    /// Destroys a `Document`. If `force` is set, the document will be
    /// destroyed and unsaved changes will be discarded and lost!
    Destroy { id: DocumentId, force: bool, reply: oneshot::Sender<Result<(), CoreCommandError>> },

    /// Gets the underlying path of a `Document`.
    GetPath { id: DocumentId, reply: oneshot::Sender<Result<Option<PathBuf>, CoreCommandError>> },

    /// Sets the underlying path of a `Document`.
    SetPath {
        id: DocumentId,
        path: Option<PathBuf>,
        reply: oneshot::Sender<Result<(), CoreCommandError>>,
    },

    /// Starts an edit history commit for a `Document`.
    StartCommit { id: DocumentId },

    /// Ends an edit history commit for a `Document`.
    EndCommit { id: DocumentId },

    /// Undoes the last committed edit for a `Document`.
    Undo { id: DocumentId },

    /// Redoes the last undone commit for a `Document`.
    HotRedo { id: DocumentId },

    /// Inserts text into a `Document`.
    Insert { id: DocumentId, pos: usize, str: String },

    /// Appends text to a `Document`.
    Append { id: DocumentId, str: String },

    /// Removes `n` bytes of text from a `Document`.
    Remove { id: DocumentId, pos: usize, n: usize },

    /// Gets the byte index where the nth line begins.
    GetLineStartByte {
        id: DocumentId,
        n: usize,
        reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    },

    /// Gets the byte index of where the nth line ends and the next line
    /// begins.
    GetLineEndByte {
        id: DocumentId,
        n: usize,
        reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    },

    /// Gets if a `Document` has unsaved changes.
    IsModified { id: DocumentId, reply: oneshot::Sender<Result<bool, CoreCommandError>> },

    /// Saves the `Document`s data to the underlying file. The amount of written
    /// bytes will be returned.
    Save { id: DocumentId, reply: oneshot::Sender<Result<usize, CoreCommandError>> },

    /// Gets a slice of text from the `Document`.
    GetSlice {
        id: DocumentId,
        range: (Bound<usize>, Bound<usize>),
        reply: oneshot::Sender<Result<String, CoreCommandError>>,
    },

    /// Gets the length of a `Document` in bytes.
    Len { id: DocumentId, reply: oneshot::Sender<Result<usize, CoreCommandError>> },

    /// Sends a batch of commands which are guaranteed to be processed in
    /// succession. Nested batched commands are skipped!
    Batch(Vec<CoreCommand>),
}

/// A trait that should be implemented by protocols to map `CoreEvent`s to
/// protocol specific events.
pub trait CoreEventMapper: Send + Sync + 'static {
    /// Maps a `CoreEvent` to a protocol specific event. This is ran
    /// _synchronously_ for all mappers and thus should not do heavy logic.
    /// Consider creating a mapper struct that communicates with the main
    /// protocol struct.
    fn map(&self, event: &CoreEvent);
}

/// An event emitted from the core. Protocols are free to process them however
/// necessary.
pub enum CoreEvent {
    /// A new `Document` was created.
    Created { id: DocumentId },

    /// A `Document` was destroyed.
    Destroyed { id: DocumentId },

    /// Text has been inserted into a `Document` at `pos`.
    Inserted { id: DocumentId, pos: usize, str: String },

    /// `n` bytes of text have been removed from a `Document` at `pos`.
    Removed { id: DocumentId, pos: usize, n: usize, str: String },

    /// A `Document` has been saved to `path` with `len` bytes written.
    Saved { id: DocumentId, path: PathBuf, len: usize },
}

pub enum CoreCommandError {
    /// An error regarding file IO occured.
    FileIo(String),
    /// The core is offline and doesn't take any requests. This may only happen
    /// during asynchronous shutdown of the application during fault-free
    /// execution.
    Offline,
    /// The core dropped the request. This may happen if the core is overloaded
    /// with requests and should not happen during fault-free execution.
    Dropped,

    /// The `Document` doesn't exist.
    NoDocument,
    /// The `Document` doesn't have an underlying file.
    NoPath,
    /// The `Document` has unsaved changes, blocking the request.
    UnsavedChanges,
}

///////////////////////////////////////////////////
// Below are implementation details of the core. //
// This is irrelevant for protocols.             //
///////////////////////////////////////////////////

pub(crate) enum InternalCoreCommand {
    /// Called after the file has been read to memory.
    FinalizeCreate {
        path: PathBuf,
        content: Result<String, String>,
        reply: oneshot::Sender<Result<DocumentId, CoreCommandError>>,
    },
    /// Called after the writing to disk finished.
    FinalizeSave {
        id: DocumentId,
        path: PathBuf,
        len: usize,
        res: Result<(), String>,
        reply: oneshot::Sender<Result<usize, CoreCommandError>>,
    },
}

pub(crate) enum CoreCommandFacade {
    Public(CoreCommand),
    Internal(InternalCoreCommand),
}
