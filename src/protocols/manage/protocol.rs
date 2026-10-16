use std::{fmt::Write, time::Duration};

use crate::{
    protocols::{
        action::ActionProtocol,
        dir::DirProtocol,
        doc_view::DocViewProtocol,
        manage::{ManageCmd, ManageEntry, ManageRenderer},
        view::ViewProtocol,
    },
    state::{DocStoreTypes, State},
    types::{Pos, render::WindowId},
    util::{self, debug_panic},
};

pub struct ManageProtocol {}

impl ManageProtocol {
    pub fn exec(s: &mut State, cmd: ManageCmd) {
        match cmd {
            ManageCmd::ReplaceWindow { window } => Self::replace_window(s, window),

            ManageCmd::Select => Self::select(s),
            ManageCmd::Close => Self::close(s),
        }
    }

    fn init(s: &mut State, move_cursor: bool) {
        let mut entries = Vec::new();
        let mut text = String::new();

        let mut docs: Vec<_> = s.doc_store.iter().map(|(id, dse)| (*id, dse)).collect();
        docs.sort_by_key(|(id, _)| id.0);

        for (id, dse) in docs {
            let modified = if dse.doc.modified { "[*]" } else { "" };
            let path = dse.doc.path.as_ref().map_or_else(
                || {
                    match dse.mode {
                        DocStoreTypes::Mode::Text => "SCRATCHPAD",
                        DocStoreTypes::Mode::Mp => "MP",
                        DocStoreTypes::Mode::Dir => "DIR",
                        DocStoreTypes::Mode::Manage => "MANAGE",
                        DocStoreTypes::Mode::Read => "READ",
                    }
                    .to_string()
                },
                |p| p.display().to_string(),
            );

            let _ = writeln!(text, "{modified} {path}");
            entries.push(ManageEntry::Doc(id));

            if let Some(views) = s.index.doc_to_views(id) {
                let mut views: Vec<_> = views.iter().copied().collect();
                views.sort_by_key(|v| v.0);

                for view in views {
                    let _ = writeln!(text, "└─ View {}", view.0);
                    entries.push(ManageEntry::View(view));
                }
            }
        }
        // Remove trailing new line.
        text.pop();

        s.p.manage.entries = entries;

        let Some((vse, dse)) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, s.p.manage.view)
        else {
            debug_panic!();
            return;
        };

        // Temporarily set the mode to not read only for the modifications.
        dse.read_only = false;

        let len = dse.doc.data.len();
        let pos = if move_cursor {
            Pos::new(0, 0)
        } else {
            util::offset_to_pos(vse.cursors.list.first().map_or(0, |c| c.offset), vse, dse)
        };

        ActionProtocol::remove(s, s.p.manage.view, 0, len);
        ActionProtocol::insert(s, s.p.manage.view, text);
        ActionProtocol::move_cursor_to_pos(s, s.p.manage.view, pos, true);
        ActionProtocol::saved(s, s.p.manage.doc);

        if let Some(dse) = s.doc_store.get_mut(&s.p.manage.doc) {
            // Reset the read only flag after the modifications.
            dse.read_only = true;
        } else {
            debug_panic!();
        }
    }

    fn replace_window(s: &mut State, window: WindowId) {
        Self::init(s, true);

        s.workspace.replace_renderer(window, Box::new(ManageRenderer::new()));
        s.index.link_window_to_view(window, s.p.manage.view);

        ViewProtocol::update(s, s.p.manage.view);
    }

    fn select(s: &mut State) {
        let Some((vse, dse)) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.manage.view)
        else {
            debug_panic!();
            return;
        };

        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let y = dse.doc.data.get_line_of_byte(offset);

        if y >= s.p.manage.entries.len() {
            debug_panic!();
            return;
        }

        let entry = s.p.manage.entries[y];

        let view = match entry {
            ManageEntry::Doc(id) => {
                s.index.doc_to_views(id).and_then(|vs| vs.iter().next()).copied()
            }
            ManageEntry::View(id) => Some(id),
        };
        let doc = match entry {
            ManageEntry::Doc(id) => id,
            ManageEntry::View(id) => {
                let Some(doc) = s.index.view_to_doc(id) else {
                    debug_panic!();
                    return;
                };

                doc
            }
        };

        // Ignore selecting oneself.
        if view == Some(s.p.manage.view) {
            return;
        }

        if let Some(view) = view
            && let Some(windows) = s.index.view_to_windows(view)
            && let Some(&window) = windows.iter().next()
        {
            DocViewProtocol::destroy_view(s, s.p.manage.view);
            s.workspace.active_window = Some(window);
            return;
        }

        let Some(dse) = s.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let window = s.workspace.active_window.unwrap();
        match dse.mode {
            DocStoreTypes::Mode::Text | DocStoreTypes::Mode::Read => {
                DocViewProtocol::replace_window(s, doc, view, window, false, None);
            }
            DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Manage => {}
            DocStoreTypes::Mode::Dir => DirProtocol::replace_window(s, window),
        }
    }

    fn close(s: &mut State) {
        let Some((vse, dse)) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.manage.view)
        else {
            debug_panic!();
            return;
        };

        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let y = dse.doc.data.get_line_of_byte(offset);

        if y >= s.p.manage.entries.len() {
            debug_panic!();
            return;
        }

        let entry = s.p.manage.entries[y];
        match entry {
            ManageEntry::Doc(id) => {
                if s.doc_store.get(&id).unwrap().doc.modified {
                    util::mp::message(
                        s,
                        "There are unsaved changes".to_string(),
                        Duration::from_secs(3),
                    );
                } else {
                    for view in s.destroy_doc(id) {
                        DocViewProtocol::destroy_view(s, view);
                    }

                    Self::init(s, false);
                }
            }
            ManageEntry::View(id) => {
                DocViewProtocol::destroy_view(s, id);

                Self::init(s, false);
            }
        }
    }
}
