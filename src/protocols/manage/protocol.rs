use std::ops::Range;
use std::time::Duration;

use crate::protocols::action::ActionProtocol;
use crate::protocols::dir::DirProtocol;
use crate::protocols::doc_view::DocViewProtocol;
use crate::protocols::manage::ManageCmd;
use crate::protocols::manage::ManageEntry;
use crate::protocols::manage::ManageRenderer;
use crate::protocols::view::ViewProtocol;
use crate::state::DocId;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewStoreTypes;
use crate::types::Pos;
use crate::types::render::WindowId;
use crate::util;

pub struct ManageProtocol {}

impl ManageProtocol {
    pub fn exec(s: &mut State, cmd: ManageCmd) {
        match cmd {
            ManageCmd::Show { window } => Self::show(s, window),

            ManageCmd::Select => Self::select(s),
            ManageCmd::Close => Self::close(s),
        }
    }

    pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
        if s.index.view_to_window(s.p.manage.view).is_none() {
            return;
        }

        let (doc, lifecycle) = match event {
            DocStoreTypes::Event::Created { doc } | DocStoreTypes::Event::Destroyed { doc } => {
                (*doc, true)
            }
            DocStoreTypes::Event::ModifiedChanged { doc, .. }
            | DocStoreTypes::Event::Written { doc, .. }
            | DocStoreTypes::Event::ModeChanged { doc, .. }
            | DocStoreTypes::Event::PathChanged { doc, .. } => (*doc, false),
            DocStoreTypes::Event::Inserted { .. } | DocStoreTypes::Event::Removed { .. } => return,
        };

        if doc == s.p.manage.doc {
            return;
        }

        if lifecycle {
            Self::init(s, false);
        } else {
            Self::update(s, doc);
        }
    }

    pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
        if s.index.view_to_window(s.p.manage.view).is_none() {
            return;
        }

        if matches!(
            event,
            ViewStoreTypes::Event::Created { .. } | ViewStoreTypes::Event::Destroyed { .. }
        ) {
            Self::init(s, false);
        }
    }

    fn init(s: &mut State, reset_cursor: bool) {
        let (selected, pos) = Self::selection(s);
        let mut docs: Vec<_> = s.doc_store.keys().copied().collect();
        docs.sort_unstable_by_key(|doc| doc.0);

        s.p.manage.entries.clear();
        s.p.manage.line_map.clear();

        let mut lines = Vec::new();
        for doc in docs {
            s.p.manage.line_map.insert(doc, s.p.manage.entries.len());
            s.p.manage.entries.push(ManageEntry::Doc(doc));

            lines.push(Self::label(s, doc).unwrap());

            if let Some(views) = s.index.doc_to_views(doc) {
                for &view in views {
                    lines.push(format!("└─ View {}", view.0));
                    s.p.manage.entries.push(ManageEntry::View(view));
                }
            }
        }

        let line = selected
            .and_then(|selected| s.p.manage.entries.iter().position(|entry| *entry == selected))
            .unwrap_or_else(|| pos.y.min(s.p.manage.entries.len().saturating_sub(1)));
        let pos = if reset_cursor { Pos::default() } else { Pos::new(pos.x, line) };

        let len = s.doc_store.get(&s.p.manage.doc).unwrap().doc.data.len();
        Self::render(s, 0..len, lines.join("\n"));
        ActionProtocol::move_cursor_to_pos(s, s.p.manage.view, pos, true);
    }

    fn show(s: &mut State, window: WindowId) {
        Self::init(s, true);

        s.replace_view(window, s.p.manage.view, Box::new(ManageRenderer::new()));

        ViewProtocol::resize(s);
    }

    fn select(s: &mut State) {
        let (Some(entry), _) = Self::selection(s) else { return };

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

        if matches!(
            dse.mode,
            DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Manage | DocStoreTypes::Mode::Picker
        ) {
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
                DocViewProtocol::show(s, doc, view, window, false, None);
            }
            DocStoreTypes::Mode::Dir => DirProtocol::show(s, window),
            DocStoreTypes::Mode::Mp | DocStoreTypes::Mode::Manage | DocStoreTypes::Mode::Picker => {
                unreachable!()
            }
        }
    }

    fn close(s: &mut State) {
        let (Some(entry), _) = Self::selection(s) else { return };
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
                }
            }
            ManageEntry::View(id) => DocViewProtocol::destroy_view(s, id),
        }
    }

    fn selection(s: &State) -> (Option<ManageEntry>, Pos) {
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.manage.view).unwrap();

        let pos = util::offset_to_pos(vse.cursors.list[0].offset, vse, dse);
        (s.p.manage.entries.get(pos.y).copied(), pos)
    }

    fn label(s: &State, doc: DocId) -> Option<String> {
        let dse = s.doc_store.get(&doc)?;

        let modified = if !dse.mode.singleton() && dse.doc.modified { "[*]" } else { "" };
        let path = dse.doc.path.as_ref().map_or_else(
            || {
                match dse.mode {
                    DocStoreTypes::Mode::Text => "SCRATCHPAD",
                    DocStoreTypes::Mode::Mp => "MP",
                    DocStoreTypes::Mode::Dir => "DIR",
                    DocStoreTypes::Mode::Manage => "MANAGE",
                    DocStoreTypes::Mode::Picker => "PICKER",
                    DocStoreTypes::Mode::Read => "READ",
                }
                .to_string()
            },
            |path| path.display().to_string().replace('\r', "\\r").replace('\n', "\\n"),
        );

        Some(format!("{modified} {path}"))
    }

    fn update(s: &mut State, doc: DocId) {
        let Some(&line) = s.p.manage.line_map.get(&doc) else { return };
        let Some(label) = Self::label(s, doc) else { return };
        let (_, pos) = Self::selection(s);

        let data = &s.doc_store.get(&s.p.manage.doc).unwrap().doc.data;
        let start = data.line_start_offset(line);
        let end = if line + 1 < s.p.manage.entries.len() {
            data.line_start_offset(line + 1) - 1
        } else {
            data.len()
        };

        if Self::render(s, start..end, label) {
            ActionProtocol::move_cursor_to_pos(s, s.p.manage.view, pos, true);
        }
    }

    fn render(s: &mut State, range: Range<usize>, text: String) -> bool {
        let dse = s.doc_store.get_mut(&s.p.manage.doc).unwrap();

        if dse.doc.data.slice(range.clone()) == text {
            return false;
        }

        // Temporarily set the mode to not read only for the modifications.
        dse.read_only = false;

        if !range.is_empty() {
            ActionProtocol::remove(s, s.p.manage.view, range.start, range.len());
        }
        if !text.is_empty() {
            ActionProtocol::insert_at(s, s.p.manage.view, text, range.start);
        }

        // Reset the read only flag after the modifications.
        let dse = s.doc_store.get_mut(&s.p.manage.doc).unwrap();
        dse.read_only = true;

        // Don't use ActionProtocol::saved to not compact manage on every
        // update.
        if dse.doc.data.memory_footprint() > dse.doc.data.len().saturating_mul(2) {
            dse.doc.data.compact();
        }
        dse.doc.modified = false;

        true
    }
}
