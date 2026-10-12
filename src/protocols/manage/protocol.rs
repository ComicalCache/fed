use std::time::Duration;

use crate::{
    protocols::{
        action::ActionProtocol,
        dir::DirProtocol,
        doc_view::DocViewProtocol,
        manage::{ManageCmd, ManageRenderer},
        pstate::PState,
        view::ViewProtocol,
    },
    state::{DocStoreTypes, ManageTypes, State},
    types::{Pos, render::WindowId},
    util::{self, debug_panic},
};

pub struct ManageProtocol {}

impl ManageProtocol {
    pub fn exec(state: &mut State, pstate: &mut PState, cmd: ManageCmd) {
        match cmd {
            ManageCmd::ReplaceWindow { window } => Self::replace_window(state, window),

            ManageCmd::Select => Self::select(state),
            ManageCmd::Close => Self::close(state, pstate),
        }
    }

    fn init(state: &mut State, move_cursor: bool) {
        let mut entries = Vec::new();
        let mut text = String::new();

        let mut docs: Vec<_> = state.doc_store.iter().map(|(id, dse)| (*id, dse)).collect();
        docs.sort_by_key(|(id, _)| id.0);

        for (id, dse) in docs {
            let modified = if dse.doc.modified { "[*]" } else { "" };
            let path =
                dse.doc.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| {
                    match dse.mode {
                        DocStoreTypes::Mode::Text => "SCRATCHPAD",
                        DocStoreTypes::Mode::Mp => "MP",
                        DocStoreTypes::Mode::Dir => "DIR",
                        DocStoreTypes::Mode::Manage => "MANAGE",
                        DocStoreTypes::Mode::Read => "READ",
                    }
                    .to_string()
                });

            text.push_str(&format!("{modified} {path}\n"));
            entries.push(ManageTypes::Entry::Doc(id));

            if let Some(views) = state.index.doc_to_views(id) {
                let mut views: Vec<_> = views.iter().cloned().collect();
                views.sort_by_key(|v| v.0);

                for view in views {
                    text.push_str(&format!("└─ View {}\n", view.0));
                    entries.push(ManageTypes::Entry::View(view));
                }
            }
        }
        // Remove trailing new line.
        text.pop();

        state.manage.entries = entries;

        let Some((vse, dse)) = State::vse_and_dse_mut(
            &mut state.view_store,
            &mut state.doc_store,
            &state.index,
            state.manage.view,
        ) else {
            debug_panic!();
            return;
        };

        // Temporarily set the mode to not read only for the modifications.
        dse.read_only = false;

        let len = dse.doc.data.len();
        let pos = if move_cursor {
            Pos::new(0, 0)
        } else {
            util::offset_to_pos(vse.cursors.list.first().map(|c| c.offset).unwrap_or(0), vse, dse)
        };

        ActionProtocol::remove(state, state.manage.view, 0, len);
        ActionProtocol::insert(state, state.manage.view, text);
        ActionProtocol::move_cursor_to_pos(state, state.manage.view, pos, true);
        ActionProtocol::saved(state, state.manage.doc);

        if let Some(dse) = state.doc_store.get_mut(&state.manage.doc) {
            // Reset the read only flag after the modifications.
            dse.read_only = true;
        } else {
            debug_panic!();
            return;
        };
    }

    fn replace_window(state: &mut State, window: WindowId) {
        Self::init(state, true);

        state.workspace.replace_renderer(window, Box::new(ManageRenderer::new(state.manage.view)));
        state.index.link_window_to_view(window, state.manage.view);

        ViewProtocol::update(state, state.manage.view);
    }

    fn select(state: &mut State) {
        let Some((vse, dse)) = State::vse_and_dse(
            &state.view_store,
            &state.doc_store,
            &state.index,
            state.manage.view,
        ) else {
            debug_panic!();
            return;
        };

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let y = dse.doc.data.get_line_of_byte(offset);

        if y >= state.manage.entries.len() {
            debug_panic!();
            return;
        }

        let entry = state.manage.entries[y];

        let view = match entry {
            ManageTypes::Entry::Doc(id) => {
                state.index.doc_to_views(id).and_then(|vs| vs.iter().next()).cloned()
            }
            ManageTypes::Entry::View(id) => Some(id),
        };
        let doc = match entry {
            ManageTypes::Entry::Doc(id) => id,
            ManageTypes::Entry::View(id) => {
                let Some(doc) = state.index.view_to_doc(id) else {
                    debug_panic!();
                    return;
                };

                doc
            }
        };

        // Ignore selecting oneself.
        if view == Some(state.manage.view) {
            return;
        }

        if let Some(view) = view
            && let Some(windows) = state.index.view_to_windows(view)
            && let Some(&window) = windows.iter().next()
        {
            DocViewProtocol::destroy_view(state, state.manage.view);
            state.workspace.active_window = Some(window);
            return;
        }

        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let window = state.workspace.active_window.unwrap();
        match dse.mode {
            DocStoreTypes::Mode::Text | DocStoreTypes::Mode::Read => {
                DocViewProtocol::replace_window(state, doc, view, window, false, None);
            }
            DocStoreTypes::Mode::Mp => {}
            DocStoreTypes::Mode::Dir => DirProtocol::replace_window(state, window),
            DocStoreTypes::Mode::Manage => {}
        }
    }

    fn close(state: &mut State, pstate: &mut PState) {
        let Some((vse, dse)) = State::vse_and_dse(
            &state.view_store,
            &state.doc_store,
            &state.index,
            state.manage.view,
        ) else {
            debug_panic!();
            return;
        };

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let y = dse.doc.data.get_line_of_byte(offset);

        if y >= state.manage.entries.len() {
            debug_panic!();
            return;
        }

        let entry = state.manage.entries[y];
        match entry {
            ManageTypes::Entry::Doc(id) => {
                if state.doc_store.get(&id).unwrap().doc.modified {
                    util::mp::message(
                        state,
                        pstate,
                        "There are unsaved changes".to_string(),
                        Duration::from_secs(3),
                    );
                } else {
                    for view in state.destroy_doc(id) {
                        DocViewProtocol::destroy_view(state, view);
                    }

                    Self::init(state, false);
                }
            }
            ManageTypes::Entry::View(id) => {
                DocViewProtocol::destroy_view(state, id);

                Self::init(state, false);
            }
        }
    }
}
