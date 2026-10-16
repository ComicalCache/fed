mod doc;
mod index;
mod view;

use std::{collections::HashSet, path::PathBuf};

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
        render::{WindowId, Workspace},
    },
    util::{self, debug_panic},
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

    /// Returns all views which contained the document.
    pub fn destroy_doc(&mut self, doc: DocId) -> HashSet<ViewId> {
        // Don't destroy internal docs.
        let Some(dse) = self.doc_store.get(&doc) else {
            debug_panic!();
            return HashSet::new();
        };

        match dse.mode {
            DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Dir | DocStoreTypes::Mode::Manage => {
                return HashSet::new();
            }
            _ => {}
        }

        self.doc_store.remove(&doc);
        let views = self.index.unlink_doc(doc);

        let _ = self.fevent_tx.send(FEvent::Doc(DocStoreTypes::Event::Destroyed { id: doc }));

        views
    }

    pub fn create_view(&mut self, doc: DocId) -> ViewId {
        let view = ViewId(self.next_view_id);
        self.next_view_id += 1;

        let entry = ViewStoreEntry::default();
        self.view_store.insert(view, entry);

        self.index.link_view_to_doc(view, doc);

        let _ = self.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::Created { id: view }));

        view
    }

    /// Returns all windows which contained the view.
    pub fn destroy_view(&mut self, view: ViewId) -> HashSet<WindowId> {
        // Don't destroy internal views, just return their windows.
        if let Some(doc) = self.index.view_to_doc(view) {
            let Some(dse) = self.doc_store.get(&doc) else {
                debug_panic!();
                return HashSet::new();
            };

            match dse.mode {
                DocStoreTypes::Mode::Mp => {
                    let Some(windows) = self.index.view_to_windows(self.p.mp.view) else {
                        debug_panic!();
                        return HashSet::new();
                    };

                    return windows.clone();
                }
                DocStoreTypes::Mode::Dir => {
                    let Some(windows) = self.index.view_to_windows(self.p.dir.view) else {
                        debug_panic!();
                        return HashSet::new();
                    };

                    return windows.clone();
                }
                DocStoreTypes::Mode::Manage => {
                    let Some(windows) = self.index.view_to_windows(self.p.manage.view) else {
                        debug_panic!();
                        return HashSet::new();
                    };

                    return windows.clone();
                }
                _ => {}
            }
        }

        self.view_store.remove(&view);

        let windows = self.index.unlink_view(view);

        let _ = self.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::Destroyed { id: view }));

        windows
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
