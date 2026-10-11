mod dir;
mod doc;
mod index;
mod manage;
mod mp;
mod view;

use std::{collections::HashSet, path::PathBuf};

pub use dir::{Dir, types as DirTypes};
pub use doc::{DocId, DocStore, DocStoreEntry, types as DocStoreTypes};
pub use manage::{Manage, types as ManageTypes};
pub use mp::{Mp, MpId, types as MpTypes};
use piece_table::PieceTable;
use tokio::sync::mpsc::UnboundedSender;
pub use view::{ViewId, ViewStore, ViewStoreEntry, types as ViewStoreTypes};

use crate::{
    debug_panic::debug_panic,
    fed::{FCmd, FEvent},
    render::{WindowId, Workspace},
    state::{ViewStoreTypes::TabWidth, index::Index},
    types::Theme,
    util,
};

pub struct State {
    next_doc_id: usize,
    next_view_id: usize,

    pub index: Index,
    pub workspace: Workspace,

    pub doc_store: DocStore,
    pub view_store: ViewStore,

    pub mp: Mp,
    pub dir: Dir,
    pub manage: Manage,

    pub theme: Theme,

    pub fevent_tx: UnboundedSender<FEvent>,
    pub async_fcmd_tx: UnboundedSender<FCmd>,
}

impl State {
    pub fn new(
        workspace: Workspace, fevent_tx: UnboundedSender<FEvent>,
        async_fcmd_tx: UnboundedSender<FCmd>,
    ) -> Self {
        let mut state = Self {
            next_doc_id: 1,
            next_view_id: 1,
            index: Index::default(),
            workspace,
            doc_store: DocStore::default(),
            view_store: ViewStore::default(),
            mp: Mp::default(),
            dir: Dir::default(),
            manage: Manage::default(),
            theme: Theme::default(),
            fevent_tx,
            async_fcmd_tx,
        };

        // Mp.
        state.mp.doc = state.create_doc(None).unwrap();
        state.mp.view = state.create_view(state.mp.doc);

        let mp_dse = state.doc_store.get_mut(&state.mp.doc).unwrap();
        mp_dse.mode = DocStoreTypes::Mode::Mp;

        let mp_vse = state.view_store.get_mut(&state.mp.view).unwrap();
        mp_vse.layout = ViewStoreTypes::Layout {
            tab_width: TabWidth::default(),
            gutter: false,
            mode_line: 0,
            replacements: ViewStoreTypes::Replacements::none(),
            rulers: Vec::new(),
        };
        mp_vse.cursors.list.clear();

        // Dir.
        state.dir.doc = state.create_doc(None).unwrap();
        state.dir.view = state.create_view(state.dir.doc);

        let dir_dse = state.doc_store.get_mut(&state.dir.doc).unwrap();
        dir_dse.mode = DocStoreTypes::Mode::Dir;
        dir_dse.read_only = true;
        dir_dse.max_cursors = Some(1);

        let dir_vse = state.view_store.get_mut(&state.dir.view).unwrap();
        dir_vse.layout = ViewStoreTypes::Layout {
            tab_width: TabWidth::default(),
            gutter: true,
            mode_line: 1,
            replacements: ViewStoreTypes::Replacements::none(),
            rulers: Vec::new(),
        };
        dir_vse.mode_line_config = DirTypes::DirModeLine::mode_line();

        // Manage.
        state.manage.doc = state.create_doc(None).unwrap();
        state.manage.view = state.create_view(state.manage.doc);

        let manage_dse = state.doc_store.get_mut(&state.manage.doc).unwrap();
        manage_dse.mode = DocStoreTypes::Mode::Manage;
        manage_dse.read_only = true;
        manage_dse.max_cursors = Some(1);

        let manage_vse = state.view_store.get_mut(&state.manage.view).unwrap();
        manage_vse.layout = ViewStoreTypes::Layout {
            tab_width: TabWidth::default(),
            gutter: true,
            mode_line: 1,
            replacements: ViewStoreTypes::Replacements::none(),
            rulers: Vec::new(),
        };
        manage_vse.mode_line_config = ManageTypes::ManageModeLine::mode_line();

        state
    }

    /// Creates a new Doc if the path is not yet opened. Returns Ok if a new Doc
    /// has been created and Err if the Doc was already present. It is always Ok
    /// if path is None.
    pub fn create_doc(&mut self, path: Option<PathBuf>) -> Result<DocId, DocId> {
        let doc = DocId(self.next_doc_id);
        self.next_doc_id += 1;

        let path = path.map(|p| util::path::normalize(p));

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
                    let Some(windows) = self.index.view_to_windows(self.mp.view) else {
                        debug_panic!();
                        return HashSet::new();
                    };

                    return windows.clone();
                }
                DocStoreTypes::Mode::Dir => {
                    let Some(windows) = self.index.view_to_windows(self.dir.view) else {
                        debug_panic!();
                        return HashSet::new();
                    };

                    return windows.clone();
                }
                DocStoreTypes::Mode::Manage => {
                    let Some(windows) = self.index.view_to_windows(self.manage.view) else {
                        debug_panic!();
                        return HashSet::new();
                    };

                    return windows.clone();
                }
                _ => {}
            }

            if matches!(
                dse.mode,
                DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Dir | DocStoreTypes::Mode::Manage
            ) {
                return HashSet::new();
            }
        }

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
