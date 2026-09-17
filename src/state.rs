mod document;
mod mini_buffer;
mod view;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

pub use document::{DocumentId, DocumentStore, DocumentStoreEntry, types as DocumentStoreTypes};
pub use mini_buffer::{MiniBufferId, MiniBufferStore, types as MiniBufferStoreTypes};
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
    pub mini_buffer_store: MiniBufferStore,

    pub window_view_map: HashMap<WindowId, ViewId>,
}

impl State {
    pub fn create_document(&mut self, path: Option<PathBuf>, data: String) -> DocumentId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        let id = DocumentId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let doc = Document::new(path, PieceTable::from(data));

        let entry = DocumentStoreEntry { doc, ..Default::default() };
        self.document_store.insert(id, entry);

        id
    }

    pub fn destroy_document(&mut self, id: DocumentId) {
        // TODO: remove doc from all views containing this doc.
        //       Should those views get a scratchpad doc or be destroyed?

        self.document_store.remove(&id);
    }

    pub fn create_view(&mut self, doc: DocumentId) -> ViewId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        let id = ViewId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let entry =
            ViewStoreEntry { doc, tab_width: ViewStoreTypes::TabWidth(4), ..Default::default() };
        self.view_store.insert(id, entry);

        id
    }

    pub fn destroy_view(&mut self, id: ViewId) { self.view_store.remove(&id); }

    pub fn active_view(&self) -> Option<ViewId> {
        self.workspace.active_window.and_then(|w| self.window_view_map.get(&w)).cloned()
    }

    pub fn view_and_doc(&self, view: ViewId) -> Option<(&ViewStoreEntry, &DocumentStoreEntry)> {
        let view = self.view_store.get(&view)?;
        let doc = self.document_store.get(&view.doc)?;

        Some((view, doc))
    }

    pub fn view_and_doc_mut(
        &mut self, view: ViewId,
    ) -> Option<(&mut ViewStoreEntry, &mut DocumentStoreEntry)> {
        let view = self.view_store.get_mut(&view)?;
        let doc = self.document_store.get_mut(&view.doc)?;

        Some((view, doc))
    }
}
