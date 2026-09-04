mod document;
mod view;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicU64, Ordering},
    },
};

pub use document::{DocumentId, DocumentStore, types as DocumentStoreTypes};
use piece_table::PieceTable;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use view::types::{Cursors, Scroll};
pub use view::{ViewId, ViewStore, types as ViewStoreTypes};

use crate::{
    protocols::io::IoCommand,
    render::{WindowId, Workspace},
    state::DocumentStoreTypes::Document,
    type_map::TypeMap,
    types::{Cursor, Pos},
};

#[derive(Default, Clone)]
pub struct State {
    pub workspace: Arc<RwLock<Workspace>>,
    pub document_store: Arc<RwLock<DocumentStore>>,
    pub view_store: Arc<RwLock<ViewStore>>,

    pub window_view_map: Arc<RwLock<HashMap<WindowId, ViewId>>>,
}

impl State {
    pub async fn create_document(
        &self, path: Option<PathBuf>, io_tx: UnboundedSender<IoCommand>,
    ) -> Result<DocumentId, String> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = DocumentId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let data = if let Some(path) = path.clone() {
            let (tx, rx) = oneshot::channel();

            io_tx.send(IoCommand::Read { path, tx }).map_err(|err| err.to_string())?;
            rx.await.map_err(|err| err.to_string())?.unwrap_or_else(|_| String::new())
        } else {
            String::new()
        };
        let doc = Document::new(path, PieceTable::from(data));

        let mut map = TypeMap::new();
        map.insert(doc);
        map.insert(DocumentStoreTypes::Decorations::default());

        self.document_store.write().unwrap().insert(id, map);

        Ok(id)
    }

    pub fn create_view(&self, doc: DocumentId) -> ViewId {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = ViewId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let mut map = TypeMap::new();
        map.insert(doc);
        map.insert(Cursors { list: vec![Cursor::default()] });
        map.insert(ViewStoreTypes::Mode::Normal);
        map.insert(Scroll(Pos::default()));
        map.insert(ViewStoreTypes::Layout::default());
        map.insert(ViewStoreTypes::Decorations::default());

        self.view_store.write().unwrap().insert(id, map);

        id
    }

    pub fn with_workspace<R>(&self, f: impl FnOnce(&Workspace) -> R) -> R {
        let workspace = self.workspace.read().unwrap();
        f(&workspace)
    }

    pub fn with_workspace_mut<R>(&self, f: impl FnOnce(&mut Workspace) -> R) -> R {
        let mut workspace = self.workspace.write().unwrap();
        f(&mut workspace)
    }

    pub fn with_view<R>(&self, view: ViewId, f: impl FnOnce(&TypeMap) -> R) -> Option<R> {
        let store = self.view_store.read().unwrap();
        let view_map = store.get(&view)?;
        Some(f(view_map))
    }

    pub fn with_view_mut<R>(&self, view: ViewId, f: impl FnOnce(&mut TypeMap) -> R) -> Option<R> {
        let mut store = self.view_store.write().unwrap();
        let view_map = store.get_mut(&view)?;
        Some(f(view_map))
    }

    pub fn with_doc<R>(&self, doc: DocumentId, f: impl FnOnce(&TypeMap) -> R) -> Option<R> {
        let store = self.document_store.read().unwrap();
        let doc_map = store.get(&doc)?;
        Some(f(doc_map))
    }

    pub fn with_doc_mut<R>(&self, doc: DocumentId, f: impl FnOnce(&mut TypeMap) -> R) -> Option<R> {
        let mut store = self.document_store.write().unwrap();
        let doc_map = store.get_mut(&doc)?;
        Some(f(doc_map))
    }

    pub fn with_window_view_map<R>(
        &self, f: impl FnOnce(RwLockReadGuard<'_, HashMap<WindowId, ViewId>>) -> R,
    ) -> Option<R> {
        let window_view_map = self.window_view_map.read().unwrap();
        Some(f(window_view_map))
    }

    pub fn with_window_view_map_mut<R>(
        &self, f: impl FnOnce(RwLockWriteGuard<'_, HashMap<WindowId, ViewId>>) -> R,
    ) -> Option<R> {
        let window_view_map = self.window_view_map.write().unwrap();
        Some(f(window_view_map))
    }
}
