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

pub use document::{DocumentId, DocumentStore, DocumentStoreEntry, types as DocumentStoreTypes};
use piece_table::PieceTable;
pub use view::{ViewId, ViewStore, ViewStoreEntry, types as ViewStoreTypes};

use crate::{
    render::{WindowId, Workspace},
    state::DocumentStoreTypes::Document,
};

#[derive(Clone)]
pub struct StateLock {
    state: Arc<RwLock<State>>,
}

impl StateLock {
    pub fn new(state: State) -> Self { Self { state: Arc::new(RwLock::new(state)) } }

    pub fn read(&self) -> RwLockReadGuard<'_, State> { self.state.read().unwrap() }

    pub fn write(&self) -> RwLockWriteGuard<'_, State> { self.state.write().unwrap() }
}

#[derive(Default)]
pub struct State {
    pub workspace: Workspace,
    pub document_store: DocumentStore,
    pub view_store: ViewStore,

    pub window_view_map: HashMap<WindowId, ViewId>,
}

impl State {
    pub fn create_document(&mut self, path: Option<PathBuf>, data: String) -> DocumentId {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = DocumentId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let doc = Document::new(path, PieceTable::from(data));

        let entry = DocumentStoreEntry { doc, ..Default::default() };
        self.document_store.insert(id, entry);

        id
    }

    pub fn create_view(&mut self, doc: DocumentId) -> ViewId {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = ViewId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let entry =
            ViewStoreEntry { doc, tab_width: ViewStoreTypes::TabWidth(4), ..Default::default() };
        self.view_store.insert(id, entry);

        id
    }

    pub fn active_view(&self) -> Option<ViewId> {
        self.workspace.active_window.and_then(|w| self.window_view_map.get(&w)).cloned()
    }

    pub fn view_and_doc(&self, view_id: ViewId) -> Option<(&ViewStoreEntry, &DocumentStoreEntry)> {
        let view = self.view_store.get(&view_id)?;
        let doc = self.document_store.get(&view.doc)?;

        Some((view, doc))
    }

    pub fn view_and_doc_mut(
        &mut self, view_id: ViewId,
    ) -> Option<(&mut ViewStoreEntry, &mut DocumentStoreEntry)> {
        let view = self.view_store.get_mut(&view_id)?;
        let doc = self.document_store.get_mut(&view.doc)?;

        Some((view, doc))
    }
}
