use std::collections::{HashMap, HashSet};

use crate::{
    render::WindowId,
    state::{DocumentId, ViewId},
};

#[derive(Default)]
pub struct Index {
    pub window_to_view: HashMap<WindowId, ViewId>,
    pub view_to_windows: HashMap<ViewId, HashSet<WindowId>>,

    pub view_to_doc: HashMap<ViewId, DocumentId>,
    pub doc_to_views: HashMap<DocumentId, HashSet<ViewId>>,
}

impl Index {
    pub fn link_window_to_view(&mut self, window: WindowId, view: ViewId) {
        self.unlink_window(window);

        self.window_to_view.insert(window, view);
        self.view_to_windows.entry(view).or_default().insert(window);
    }

    pub fn unlink_window(&mut self, window: WindowId) {
        if let Some(view) = self.window_to_view.remove(&window) {
            if let Some(windows) = self.view_to_windows.get_mut(&view) {
                windows.remove(&window);

                if windows.is_empty() {
                    self.view_to_windows.remove(&view);
                }
            }
        }
    }

    pub fn link_view_to_doc(&mut self, view: ViewId, doc: DocumentId) {
        if let Some(doc) = self.view_to_doc.remove(&view) {
            if let Some(views) = self.doc_to_views.get_mut(&doc) {
                views.remove(&view);

                if views.is_empty() {
                    self.doc_to_views.remove(&doc);
                }
            }
        }

        self.view_to_doc.insert(view, doc);
        self.doc_to_views.entry(doc).or_default().insert(view);
    }

    /// Returns all windows which contained the view.
    pub fn unlink_view(&mut self, view: ViewId) -> HashSet<WindowId> {
        if let Some(doc) = self.view_to_doc.remove(&view) {
            if let Some(views) = self.doc_to_views.get_mut(&doc) {
                views.remove(&view);

                if views.is_empty() {
                    self.doc_to_views.remove(&doc);
                }
            }
        }

        let windows = self.view_to_windows.remove(&view).unwrap_or_default();
        for window in &windows {
            self.window_to_view.remove(window);
        }

        windows
    }

    /// Returns all views which contained the document.
    pub fn unlink_doc(&mut self, doc: DocumentId) -> HashSet<ViewId> {
        let views = self.doc_to_views.remove(&doc).unwrap_or_default();
        for view in &views {
            self.view_to_doc.remove(view);
        }

        views
    }

    pub fn window_to_view(&self, window: WindowId) -> Option<ViewId> {
        self.window_to_view.get(&window).copied()
    }

    pub fn view_to_windows(&self, view: ViewId) -> Option<&HashSet<WindowId>> {
        self.view_to_windows.get(&view)
    }

    pub fn view_to_doc(&self, view: ViewId) -> Option<DocumentId> {
        self.view_to_doc.get(&view).copied()
    }

    pub fn doc_to_views(&self, doc: DocumentId) -> Option<&HashSet<ViewId>> {
        self.doc_to_views.get(&doc)
    }
}
