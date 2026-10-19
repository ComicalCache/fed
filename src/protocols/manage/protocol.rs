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
    util,
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

    pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
        match event {
            DocStoreTypes::Event::Created { .. } | DocStoreTypes::Event::Destroyed { .. } => {}
            DocStoreTypes::Event::Written { .. }
            | DocStoreTypes::Event::Inserted { .. }
            | DocStoreTypes::Event::Removed { .. }
            | DocStoreTypes::Event::ModeChanged { .. }
            | DocStoreTypes::Event::PathChanged { .. } => return,
        }

        if s.index.view_to_window(s.p.manage.view).is_some() {
            Self::init(s, false);
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
                for &view in views {
                    let _ = writeln!(text, "└─ View {}", view.0);
                    entries.push(ManageEntry::View(view));
                }
            }
        }
        // Remove trailing new line.
        text.pop();

        s.p.manage.entries = entries;

        let (vse, dse) =
            State::vse_and_dse_mut(&mut s.view_store, &mut s.doc_store, &s.index, s.p.manage.view)
                .unwrap();

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

        s.doc_store.get_mut(&s.p.manage.doc).unwrap().read_only = true;
    }

    fn replace_window(s: &mut State, window: WindowId) {
        Self::init(s, true);

        s.replace_view(window, s.p.manage.view, Box::new(ManageRenderer::new()));

        ViewProtocol::resize(s);
    }

    fn select(s: &mut State) {
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.manage.view).unwrap();

        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let y = dse.doc.data.get_line_of_byte(offset);
        let entry = s.p.manage.entries[y];

        let view = match entry {
            ManageEntry::Doc(id) => s.index.doc_to_views(id).map(|v| v[0]),
            ManageEntry::View(id) => Some(id),
        };
        let doc = match entry {
            ManageEntry::Doc(id) => id,
            ManageEntry::View(id) => s.index.view_to_doc(id).unwrap(),
        };

        // Ignore selecting oneself.
        if view == Some(s.p.manage.view) {
            return;
        }

        let dse = s.doc_store.get(&doc).unwrap();

        if matches!(dse.mode, DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Manage) {
            return;
        }

        if let Some(view) = view
            && let Some(window) = s.index.view_to_window(view)
        {
            ActionProtocol::set_active_window(s, Some(window));
            return;
        }

        let window = s.index.view_to_window(s.p.manage.view).unwrap();
        match dse.mode {
            DocStoreTypes::Mode::Text | DocStoreTypes::Mode::Read => {
                DocViewProtocol::replace_window(s, doc, view, window, false, None);
            }
            DocStoreTypes::Mode::Dir => DirProtocol::replace_window(s, window),
            DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Manage => unreachable!(),
        }
    }

    fn close(s: &mut State) {
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.manage.view).unwrap();

        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let y = dse.doc.data.get_line_of_byte(offset);
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
                    s.destroy_doc(id);

                    ViewProtocol::resize(s);

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
