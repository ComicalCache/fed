use std::{path::PathBuf, time::Duration};

use tokio::sync::oneshot;

use crate::{
    fed::FCmd,
    protocols::{
        action::ActionProtocol,
        doc_view::{DocViewCmd, DocViewRenderer},
        io::IoProtocol,
        view::ViewProtocol,
    },
    state::{DocId, State, ViewId},
    types::{
        Pos, Rect, RectSplit,
        render::{WindowId, ZLayer},
    },
    util,
};

pub struct DocViewProtocol {}

impl DocViewProtocol {
    pub fn exec(s: &mut State, cmd: DocViewCmd) {
        match cmd {
            DocViewCmd::OpenFile { window, path, force } => Self::open_file(s, window, path, force),
            DocViewCmd::OpenFileCompletion { window, id, path, force, res } => {
                Self::open_file_completion(s, window, id, path, force, res);
            }
            DocViewCmd::CreateTile { doc, view, split_window, direction, raw, tx } => {
                Self::create_tile(s, doc, view, split_window, direction, raw, tx);
            }
            DocViewCmd::CreateFloating { doc, view, rect, z, raw, tx } => {
                Self::create_floating(s, doc, view, rect, z, raw, tx);
            }
            DocViewCmd::ReplaceWindow { doc, view, window, raw, tx } => {
                Self::replace_window(s, doc, view, window, raw, tx);
            }
            DocViewCmd::DestroyView { view } => Self::destroy_view(s, view),
        }
    }

    pub fn open_file(s: &mut State, window: WindowId, path: PathBuf, force: bool) {
        s.p.doc_view.pending.remove(&window);

        let path = util::path::normalize(path);
        let doc = s
            .doc_store
            .iter()
            .find(|(_, dse)| dse.doc.path.as_ref() == Some(&path))
            .map(|(&doc, _)| doc);

        if let Some(doc) = doc {
            Self::show_doc(s, doc, window, force);
            return;
        }

        let id = s.p.doc_view.revision;
        s.p.doc_view.revision += 1;

        let views = s.index.window_to_views(window).unwrap().clone();
        s.p.doc_view.pending.insert(window, (id, views));

        let tx = s.async_fcmd_tx.clone();
        IoProtocol::read(
            path.clone(),
            Box::new(move |res| {
                Box::pin(async move {
                    let _ = tx.send(FCmd::DocView(DocViewCmd::OpenFileCompletion {
                        window,
                        id,
                        path,
                        force,
                        res,
                    }));
                })
            }),
        );
    }

    fn open_file_completion(
        s: &mut State, window: WindowId, id: u64, path: PathBuf, force: bool,
        result: Result<String, String>,
    ) {
        if s.p.doc_view.pending.get(&window).is_none_or(|(revision, _)| *revision != id) {
            return;
        }

        let (_, views) = s.p.doc_view.pending.remove(&window).unwrap();

        // Window was changed since open file was issued, ignore.
        if s.index.window_to_views(window) != Some(&views) {
            return;
        }

        let text = match result {
            Ok(text) => text,
            Err(err) => {
                util::mp::message(s, format!("Error reading file: {err}"), Duration::from_secs(3));
                return;
            }
        };

        match s.create_doc(Some(path)) {
            Ok(doc) => {
                let view = Self::replace_window(s, doc, None, window, false, None);
                ActionProtocol::insert(s, view, text);
                ActionProtocol::move_cursor_to_pos(s, view, Pos::default(), true);
                ActionProtocol::saved(s, doc);
            }
            Err(doc) => Self::show_doc(s, doc, window, force),
        }
    }

    pub fn create_tile(
        s: &mut State, doc: DocId, view: Option<ViewId>, split_window: WindowId,
        direction: RectSplit, raw: bool, tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    ) -> (ViewId, WindowId) {
        let view = Self::prepare_view(s, doc, view, raw);
        let window = s.workspace.create_tile(
            split_window,
            direction,
            Box::new(DocViewRenderer::new(view, raw)),
        );
        s.attach_view(window, view);

        ViewProtocol::resize(s);

        if let Some(tx) = tx {
            let _ = tx.send((view, window));
        }

        (view, window)
    }

    pub fn create_floating(
        s: &mut State, doc: DocId, view: Option<ViewId>, rect: Rect, z: ZLayer, raw: bool,
        tx: Option<oneshot::Sender<(ViewId, WindowId)>>,
    ) -> (ViewId, WindowId) {
        let view = Self::prepare_view(s, doc, view, raw);
        let window =
            s.workspace.create_floating(rect, z, Box::new(DocViewRenderer::new(view, raw)));
        s.attach_view(window, view);
        ViewProtocol::resize(s);

        if let Some(tx) = tx {
            let _ = tx.send((view, window));
        }

        (view, window)
    }

    pub fn replace_window(
        s: &mut State, doc: DocId, view: Option<ViewId>, window: WindowId, raw: bool,
        tx: Option<oneshot::Sender<ViewId>>,
    ) -> ViewId {
        s.p.doc_view.pending.remove(&window);

        let view = Self::prepare_view(s, doc, view, raw);
        s.replace_view(window, view, Box::new(DocViewRenderer::new(view, raw)));
        ViewProtocol::resize(s);

        if let Some(tx) = tx {
            let _ = tx.send(view);
        }

        view
    }

    pub fn destroy_view(s: &mut State, view: ViewId) {
        if s.destroy_view(view).is_some() {
            ViewProtocol::resize(s);
        }
    }

    fn prepare_view(s: &mut State, doc: DocId, view: Option<ViewId>, raw: bool) -> ViewId {
        let view = view.unwrap_or_else(|| s.create_view(doc));

        if raw {
            let vse = s.view_store.get_mut(&view).unwrap();
            vse.layout.gutter = false;
            vse.layout.mode_line = 0;
        }

        view
    }

    fn show_doc(s: &mut State, doc: DocId, window: WindowId, force: bool) {
        let view = if force {
            None
        } else {
            let views = s.index.doc_to_views(doc);

            if let Some(window) =
                views.and_then(|v| v.iter().find_map(|v| s.index.view_to_window(*v)))
            {
                ActionProtocol::set_active_window(s, Some(window));
                return;
            }

            views.map(|views| views[0])
        };

        Self::replace_window(s, doc, view, window, false, None);
    }
}
