mod doc;
mod index;
mod view;

use std::path::PathBuf;

pub use doc::{DocId, DocStore, DocStoreEntry, types as DocStoreTypes};
use piece_table::PieceTable;
use tokio::sync::mpsc::UnboundedSender;
pub use view::{ViewId, ViewStore, ViewStoreEntry, types as ViewStoreTypes};

use crate::{
    fed::{FCmd, FEvent},
    modes,
    protocols::pstate::PState,
    state::index::Index,
    types::{
        Pos, Rect, Theme,
        render::{Renderer, WindowId, Workspace},
    },
    util,
};

pub struct State {
    next_doc_id: usize,
    next_view_id: usize,

    pub p: PState,

    pub index: Index,

    pub workspace: Workspace,

    pub doc_store: DocStore,
    pub view_store: ViewStore,

    pub theme: Theme,

    pub fevent_tx: UnboundedSender<FEvent>,
    pub async_fcmd_tx: UnboundedSender<FCmd>,
}

impl State {
    pub fn new(
        width: usize, height: usize, fevent_tx: UnboundedSender<FEvent>,
        async_fcmd_tx: UnboundedSender<FCmd>,
    ) -> Self {
        let mut s = Self {
            next_doc_id: 1,
            next_view_id: 1,
            p: PState::new(width, height),
            index: Index::default(),
            workspace: Workspace::new(Rect::new(Pos::default(), width, height)),
            doc_store: DocStore::default(),
            view_store: ViewStore::default(),
            theme: Theme::default(),
            fevent_tx,
            async_fcmd_tx,
        };

        modes::mp::setup::setup(&mut s);
        modes::dir::setup::setup(&mut s);
        modes::manage::setup::setup(&mut s);

        s
    }

    /// Creates a new Doc if the path is not yet opened. Returns Ok if a new Doc
    /// has been created and Err if the Doc was already present. It is always Ok
    /// if path is None.
    pub fn create_doc(&mut self, path: Option<PathBuf>) -> Result<DocId, DocId> {
        let doc = DocId(self.next_doc_id);
        self.next_doc_id += 1;

        let path = path.map(util::path::normalize);

        if let Some(path) = &path
            && let Some((&doc, _)) =
                self.doc_store.iter().find(|(_, dse)| dse.doc.path.as_ref() == Some(path))
        {
            return Err(doc);
        }

        let entry = DocStoreEntry {
            doc: DocStoreTypes::Doc::new(path, PieceTable::from("")),
            ..Default::default()
        };
        self.doc_store.insert(doc, entry);

        let _ = self.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Created { id: doc }));

        Ok(doc)
    }

    pub fn destroy_doc(&mut self, doc: DocId) {
        if self.doc_store.get(&doc).unwrap().mode.singleton() {
            return;
        }

        let views = self.index.doc_to_views(doc).cloned().unwrap_or_default();
        for view in views {
            self.destroy_view(view);
        }

        self.doc_store.remove(&doc);

        let _ = self.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Destroyed { id: doc }));
    }

    pub fn create_view(&mut self, doc: DocId) -> ViewId {
        let dse = self.doc_store.get(&doc).unwrap();
        assert!(!dse.mode.singleton() || self.index.doc_to_views(doc).is_none());

        let view = ViewId(self.next_view_id);
        self.next_view_id += 1;

        let entry = ViewStoreEntry::default();
        self.view_store.insert(view, entry);

        self.index.link_view_to_doc(view, doc);

        let _ = self.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::Created { id: view }));

        view
    }

    /// Returns the window which contained the view.
    pub fn destroy_view(&mut self, view: ViewId) -> Option<WindowId> {
        self.view_store.get(&view)?;

        let doc = self.index.view_to_doc(view).unwrap();
        if self.doc_store.get(&doc).unwrap().mode.singleton() {
            return None;
        }

        self.view_store.remove(&view);

        let window = self.index.unlink_view(view);
        if let Some(window) = window
            && self.index.window_to_views(window).is_none()
        {
            self.workspace.destroy_window(window);
        }

        let _ = self.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::Destroyed { id: view }));

        window
    }

    pub fn attach_view(&mut self, window: WindowId, view: ViewId) {
        assert!(self.workspace.get_rect(window).is_some());
        assert!(self.view_store.contains_key(&view));
        assert!(self.doc_store.contains_key(&self.index.view_to_doc(view).unwrap()));

        if let Some(previous) = self.index.link_window_to_view(window, view)
            && previous != window
            && self.index.window_to_views(previous).is_none()
        {
            if self.workspace.active_window == Some(previous) {
                self.workspace.active_window = Some(window);
            }

            self.workspace.destroy_window(previous);
        }
    }

    pub fn replace_view(&mut self, window: WindowId, view: ViewId, renderer: Box<dyn Renderer>) {
        self.workspace.replace_renderer(window, renderer);
        self.index.unlink_window(window);

        self.attach_view(window, view);
    }

    pub fn destroy_window(&mut self, window: WindowId) {
        assert!(self.workspace.get_rect(window).is_some());

        self.index.unlink_window(window);
        self.workspace.destroy_window(window);
    }

    pub fn active_views(&self) -> Option<&Vec<ViewId>> {
        self.workspace.active_window.and_then(|window| self.index.window_to_views(window))
    }

    pub fn active_view(&self) -> Option<ViewId> {
        let views = self.active_views()?;

        if views.len() != 1 {
            return None;
        }

        Some(views[0])
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
