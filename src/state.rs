mod doc;
mod index;
mod mini_buffer;
mod view;

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

pub use doc::{DocId, DocStore, DocStoreEntry, types as DocStoreTypes};
pub use mini_buffer::{MiniBufferId, MiniBufferStore, types as MiniBufferStoreTypes};
use piece_table::PieceTable;
use tokio::sync::broadcast;
pub use view::{ViewId, ViewStore, ViewStoreEntry, types as ViewStoreTypes};

use crate::{
    render::{WindowId, Workspace},
    state::{DocStoreTypes::Doc, index::Index},
    types::Theme,
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

pub struct State {
    pub index: Index,
    pub workspace: Workspace,

    pub doc_store: DocStore,
    pub view_store: ViewStore,
    pub mini_buffer_store: MiniBufferStore,

    pub doc_event_tx: broadcast::Sender<DocStoreTypes::Event>,
    pub view_event_tx: broadcast::Sender<ViewStoreTypes::Event>,

    pub theme: Theme,
}

impl State {
    pub fn new(
        workspace: Workspace, doc_event_tx: broadcast::Sender<DocStoreTypes::Event>,
        view_event_tx: broadcast::Sender<ViewStoreTypes::Event>,
    ) -> Self {
        Self {
            index: Index::default(),
            workspace,
            doc_store: DocStore::default(),
            view_store: ViewStore::default(),
            mini_buffer_store: MiniBufferStore::default(),
            doc_event_tx,
            view_event_tx,
            theme: Theme::default(),
        }
    }

    pub fn create_doc(&mut self, path: Option<PathBuf>, data: String) -> DocId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        let doc = DocId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let entry =
            DocStoreEntry { doc: Doc::new(path, PieceTable::from(data)), ..Default::default() };
        self.doc_store.insert(doc, entry);

        let _ = self.doc_event_tx.send(DocStoreTypes::Event::Created { id: doc });

        doc
    }

    /// Returns all views which contained the document.
    pub fn destroy_doc(&mut self, doc: DocId) -> HashSet<ViewId> {
        self.doc_store.remove(&doc);
        let views = self.index.unlink_doc(doc);

        let _ = self.doc_event_tx.send(DocStoreTypes::Event::Destroyed { id: doc });

        views
    }

    pub fn create_view(&mut self, doc: DocId) -> ViewId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        let view = ViewId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let entry = ViewStoreEntry { tab_width: ViewStoreTypes::TabWidth(4), ..Default::default() };
        self.view_store.insert(view, entry);

        self.index.link_view_to_doc(view, doc);

        view
    }

    /// Returns all windows which contained the view.
    pub fn destroy_view(&mut self, view: ViewId) -> HashSet<WindowId> {
        self.view_store.remove(&view);

        self.index.unlink_view(view)
    }

    pub fn active_view(&self) -> Option<ViewId> {
        self.workspace.active_window.and_then(|w| self.index.window_to_view(w))
    }

    pub fn vse_and_dse<'a>(
        view_store: &'a ViewStore, doc_store: &'a DocStore, index: &Index, view: ViewId,
    ) -> Option<(&'a ViewStoreEntry, &'a DocStoreEntry)> {
        let vse = view_store.get(&view)?;
        let dse = doc_store.get(&index.view_to_doc(view)?)?;

        Some((vse, dse))
    }

    pub fn vse_and_dse_mut<'a>(
        view_store: &'a mut ViewStore, doc_store: &'a mut DocStore, index: &Index, view: ViewId,
    ) -> Option<(&'a mut ViewStoreEntry, &'a mut DocStoreEntry)> {
        let vse = view_store.get_mut(&view)?;
        let dse = doc_store.get_mut(&index.view_to_doc(view)?)?;

        Some((vse, dse))
    }
}
