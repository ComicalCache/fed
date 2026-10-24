use std::collections::HashMap;

use crate::state::DocId;
use crate::state::ViewId;
use crate::types::render::WindowId;

/// View lists are nonempty, sorted by ID, and contain no duplicates.
#[derive(Default)]
pub struct Index {
    window_to_views: HashMap<WindowId, Vec<ViewId>>,
    view_to_window: HashMap<ViewId, WindowId>,

    view_to_doc: HashMap<ViewId, DocId>,
    doc_to_views: HashMap<DocId, Vec<ViewId>>,
}

impl Index {
    #[must_use]
    pub fn window_to_views(&self, window: WindowId) -> Option<&Vec<ViewId>> {
        self.window_to_views.get(&window)
    }

    #[must_use]
    pub fn view_to_window(&self, view: ViewId) -> Option<WindowId> {
        self.view_to_window.get(&view).copied()
    }

    #[must_use]
    pub fn view_to_doc(&self, view: ViewId) -> Option<DocId> {
        self.view_to_doc.get(&view).copied()
    }

    #[must_use]
    pub fn doc_to_views(&self, doc: DocId) -> Option<&Vec<ViewId>> { self.doc_to_views.get(&doc) }

    #[must_use]
    pub(super) fn link_window_to_view(
        &mut self, window: WindowId, view: ViewId,
    ) -> Option<WindowId> {
        assert!(self.view_to_doc.contains_key(&view));

        let previous = self.unlink_view_from_window(view);
        self.view_to_window.insert(view, window);

        let views = self.window_to_views.entry(window).or_default();
        let position = views.binary_search_by_key(&view.0, |v| v.0).unwrap_err();
        views.insert(position, view);

        previous
    }

    pub(super) fn link_view_to_doc(&mut self, view: ViewId, doc: DocId) {
        assert!(!self.view_to_doc.contains_key(&view));

        self.view_to_doc.insert(view, doc);

        let views = self.doc_to_views.entry(doc).or_default();
        let position = views.binary_search_by_key(&view.0, |v| v.0).unwrap_err();
        views.insert(position, view);
    }

    pub(super) fn unlink_window(&mut self, window: WindowId) {
        if let Some(views) = self.window_to_views.remove(&window) {
            for view in views {
                assert!(self.view_to_window.remove(&view) == Some(window));
            }
        }
    }

    #[must_use]
    pub(super) fn unlink_view(&mut self, view: ViewId) -> Option<WindowId> {
        let doc = self.view_to_doc.remove(&view).unwrap();
        let views = self.doc_to_views.get_mut(&doc).unwrap();
        let position = views.binary_search_by_key(&view.0, |v| v.0).unwrap();
        views.remove(position);
        if views.is_empty() {
            self.doc_to_views.remove(&doc);
        }

        self.unlink_view_from_window(view)
    }

    fn unlink_view_from_window(&mut self, view: ViewId) -> Option<WindowId> {
        let window = self.view_to_window.remove(&view)?;
        let views = self.window_to_views.get_mut(&window).unwrap();
        let position = views.binary_search_by_key(&view.0, |v| v.0).unwrap();
        views.remove(position);

        if views.is_empty() {
            self.window_to_views.remove(&window);
        }

        Some(window)
    }
}
