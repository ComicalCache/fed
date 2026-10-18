use std::collections::{HashMap, HashSet};

use crate::{
    state::{DocId, ViewId},
    types::render::WindowId,
    util::debug_panic,
};

#[derive(Default)]
pub struct Index {
    window_to_views: HashMap<WindowId, HashSet<ViewId>>,
    view_to_window: HashMap<ViewId, WindowId>,

    view_to_doc: HashMap<ViewId, DocId>,
    doc_to_views: HashMap<DocId, HashSet<ViewId>>,
}

impl Index {
    /// Returns the window the view was previously linked to.
    #[must_use]
    pub fn link_window_to_view(&mut self, window: WindowId, view: ViewId) -> Option<WindowId> {
        let prev_window = self.view_to_window.insert(view, window);

        if let Some(prev_window) = prev_window
            && prev_window != window
        {
            if let Some(views) = self.window_to_views.get_mut(&prev_window) {
                views.remove(&view);

                if views.is_empty() {
                    self.window_to_views.remove(&prev_window);
                }
            } else {
                debug_panic!();
            }
        }

        self.window_to_views.entry(window).or_default().insert(view);

        prev_window
    }

    pub fn link_view_to_doc(&mut self, view: ViewId, doc: DocId) {
        if let Some(doc) = self.view_to_doc.remove(&view) {
            if let Some(views) = self.doc_to_views.get_mut(&doc) {
                views.remove(&view);

                if views.is_empty() {
                    self.doc_to_views.remove(&doc);
                }
            } else {
                debug_panic!();
            }
        }

        self.view_to_doc.insert(view, doc);
        self.doc_to_views.entry(doc).or_default().insert(view);
    }

    pub fn unlink_window(&mut self, window: WindowId) {
        if let Some(views) = self.window_to_views.remove(&window) {
            for view in views {
                let prev_window = self.view_to_window.remove(&view);
                debug_assert!(prev_window == Some(window));
            }
        }
    }

    /// Returns the window which contained the view.
    #[must_use]
    pub fn unlink_view(&mut self, view: ViewId) -> Option<WindowId> {
        if let Some(doc) = self.view_to_doc.remove(&view) {
            if let Some(views) = self.doc_to_views.get_mut(&doc) {
                views.remove(&view);

                if views.is_empty() {
                    self.doc_to_views.remove(&doc);
                }
            } else {
                debug_panic!();
            }
        }

        self.unlink_view_from_window(view)
    }

    /// Detaches the view while preserving its document association.
    pub fn unlink_view_from_window(&mut self, view: ViewId) -> Option<WindowId> {
        let window = self.view_to_window.remove(&view);
        if let Some(window) = window {
            if let Some(views) = self.window_to_views.get_mut(&window) {
                views.remove(&view);

                if views.is_empty() {
                    self.window_to_views.remove(&window);
                }
            } else {
                debug_panic!();
            }
        }

        window
    }

    /// Returns all views which contained the document.
    pub fn unlink_doc(&mut self, doc: DocId) -> HashSet<ViewId> {
        let views = self.doc_to_views.remove(&doc).unwrap_or_default();
        for view in &views {
            self.view_to_doc.remove(view);
        }

        views
    }

    pub fn window_to_views(&self, window: WindowId) -> Option<&HashSet<ViewId>> {
        let views = self.window_to_views.get(&window);

        if cfg!(debug_assertions)
            && let Some(views) = views
        {
            for view in views {
                debug_assert!(self.view_to_window.get(view).is_some_and(|&w| w == window));
            }
        }

        views
    }

    pub fn view_to_window(&self, view: ViewId) -> Option<WindowId> {
        let window = self.view_to_window.get(&view).cloned();

        if cfg!(debug_assertions)
            && let Some(window) = window
        {
            debug_assert!(self.window_to_views.get(&window).is_some_and(|v| v.contains(&view)));
        }

        window
    }

    pub fn view_to_doc(&self, view: ViewId) -> Option<DocId> {
        let doc = self.view_to_doc.get(&view).copied();

        if cfg!(debug_assertions)
            && let Some(doc) = doc
        {
            let windows = self.doc_to_views.get(&doc);
            debug_assert!(windows.is_some_and(|w| w.contains(&view)));
        }

        doc
    }

    pub fn doc_to_views(&self, doc: DocId) -> Option<&HashSet<ViewId>> {
        let views = self.doc_to_views.get(&doc);

        if cfg!(debug_assertions)
            && let Some(views) = views
        {
            for view in views {
                debug_assert_eq!(self.view_to_doc.get(view), Some(&doc));
            }
        }

        views
    }
}
