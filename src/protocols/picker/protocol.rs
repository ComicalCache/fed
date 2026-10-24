use piece_table::Slice;

use crate::fed::FEvent;
use crate::protocols::action::ActionProtocol;
use crate::protocols::mp::MpKind;
use crate::protocols::picker::PCallback;
use crate::protocols::picker::PickerCmd;
use crate::protocols::picker::PickerEntry;
use crate::protocols::picker::PickerRenderer;
use crate::protocols::view::ViewProtocol;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Motion;
use crate::types::Pos;
use crate::types::Rect;
use crate::types::render::ZLayer;

pub struct PickerProtocol {}

impl PickerProtocol {
    pub fn exec(s: &mut State, cmd: PickerCmd) {
        match cmd {
            PickerCmd::Open { width, height, entries, callback } => {
                Self::open(s, width, height, entries, callback);
            }
            PickerCmd::Close => Self::close(s, None),

            PickerCmd::FilterList => Self::filter_list(s),
            PickerCmd::ChangeSelection(motion) => Self::change_selection(s, motion),
            PickerCmd::ScrollList(motion) => Self::scroll_list(s, motion),
            PickerCmd::Select => Self::select(s),

            PickerCmd::Resize { width, height } => Self::resize(s, width, height),
        }
    }

    pub fn open(
        s: &mut State, width: usize, height: usize, entries: Vec<Box<dyn PickerEntry>>,
        callback: PCallback,
    ) {
        if s.p.picker.window.is_some() || s.p.mp.kind == MpKind::Prompt {
            callback(s, None);
            return;
        }

        s.p.picker.pref_width = width;
        s.p.picker.pref_height = height;

        let window = s.workspace.create_floating(
            Rect::default(),
            ZLayer::Picker,
            Box::new(PickerRenderer::new()),
        );
        s.attach_view(window, s.p.picker.input_view);
        s.attach_view(window, s.p.picker.list_view);

        s.p.picker.prev_window = s.workspace.active_window;
        s.p.picker.window = Some(window);

        s.workspace.active_window = Some(window);

        s.p.picker.display_text = entries
            .iter()
            .map(|e| e.to_string().replace('\r', "\\r").replace('\n', "\\n"))
            .collect();
        s.p.picker.search_text = s.p.picker.display_text.iter().map(|l| l.to_lowercase()).collect();
        s.p.picker.entries = entries;
        s.p.picker.offset = 0;
        s.p.picker.pcallback = Some(callback);

        Self::set_text(s, s.p.picker.input_view, String::new());

        let rect = s.workspace.rect();
        Self::resize(s, rect.width, rect.height);
        Self::filter_list(s);
    }

    pub fn close(s: &mut State, selected: Option<Box<dyn PickerEntry>>) {
        let Some(window) = s.p.picker.window.take() else { return };

        let callback = s.p.picker.pcallback.take().unwrap();
        let previous = s.p.picker.prev_window.take();
        let was_active = s.workspace.active_window == Some(window);

        s.destroy_window(window);

        let restore = previous
            .filter(|window| s.workspace.get_rect(*window).is_some())
            .or_else(|| s.workspace.tiles().into_iter().next());

        if was_active {
            s.workspace.active_window = restore;
        }
        if s.p.mp.prev_window == Some(window) {
            s.p.mp.prev_window = restore;
        }

        s.p.picker.entries.clear();
        s.p.picker.display_text.clear();
        s.p.picker.search_text.clear();
        s.p.picker.filtered.clear();
        s.p.picker.offset = 0;

        for view in [s.p.picker.input_view, s.p.picker.list_view] {
            s.view_store.get_mut(&view).unwrap().rect = Rect::default();
            Self::set_text(s, view, String::new());
        }

        callback(s, selected);
    }

    pub fn filter_list(s: &mut State) {
        if s.p.picker.window.is_none() {
            return;
        }

        let dse = s.doc_store.get(&s.p.picker.input_doc).unwrap();
        let query = dse.doc.data.slice(0..dse.doc.data.len()).to_lowercase();

        let picker = &mut s.p.picker;
        picker.filtered.clear();

        if query.is_empty() {
            picker.filtered.extend(0..picker.entries.len());
        } else {
            picker.filtered.extend(
                picker
                    .search_text
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, label)| label.contains(&query).then_some(idx)),
            );
        }
        picker.offset = 0;

        Self::update_list(s, 0);
    }

    pub fn change_selection(s: &mut State, motion: Motion) {
        if s.p.picker.window.is_none() || s.p.picker.filtered.is_empty() {
            return;
        }

        let view = s.p.picker.list_view;
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let x = vse.scroll.x;
        let y = Self::selection(s);
        let height = vse.layout(dse).content.height;

        let last = s.p.picker.filtered.len() - 1;
        let y = match motion {
            Motion::Up => y.saturating_sub(1).min(last),
            Motion::Down => y.saturating_add(1).min(last),
            _ => return,
        };

        let start = s.p.picker.offset;
        if y < start || y >= start + height {
            Self::update_list(s, y);
        } else {
            ActionProtocol::move_cursor_to_pos(s, view, Pos::new(x, y - start), true);
        }
    }

    pub fn scroll_list(s: &mut State, motion: Motion) {
        if s.p.picker.window.is_none() || s.p.picker.filtered.is_empty() {
            return;
        }

        let mut scroll = *s.view_store.get(&s.p.picker.list_view).unwrap().scroll;
        match motion {
            Motion::Left => scroll.x = scroll.x.saturating_sub(1),
            Motion::Right => scroll.x = scroll.x.saturating_add(1),
            _ => return,
        }

        ViewProtocol::scroll_to(s, s.p.picker.list_view, scroll);
    }

    pub fn select(s: &mut State) {
        if s.p.picker.window.is_none() {
            return;
        }

        let y = Self::selection(s);
        let Some(&index) = s.p.picker.filtered.get(y) else { return };

        let selected = s.p.picker.entries.remove(index);
        Self::close(s, Some(selected));
    }

    pub fn resize(s: &mut State, width: usize, height: usize) {
        let Some(window) = s.p.picker.window else { return };
        let selection = Self::selection(s);

        let w = s.p.picker.pref_width.max(3).min(width);
        let h = s.p.picker.pref_height.max(5).min(height);

        s.workspace.reposition_floating(
            window,
            Pos::new(width.saturating_sub(w) / 2, height.saturating_sub(h) / 2),
        );
        s.workspace.resize_floating(window, w, h);

        let (input, list) = if w >= 3 && h >= 5 {
            (Rect::new(Pos::new(1, 1), w - 2, 1), Rect::new(Pos::new(1, 3), w - 2, h - 4))
        } else {
            (Rect::default(), Rect::default())
        };

        s.view_store.get_mut(&s.p.picker.input_view).unwrap().rect = input;
        s.view_store.get_mut(&s.p.picker.list_view).unwrap().rect = list;
        Self::update_list(s, selection);

        for view in [s.p.picker.input_view, s.p.picker.list_view] {
            ViewProtocol::scroll_if_needed(s, view);
            let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::Resized { id: view }));
        }
    }

    fn selection(s: &State) -> usize {
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.list_view)
                .unwrap();

        s.p.picker.offset + dse.doc.data.get_line_of_byte(vse.cursors.list[0].offset)
    }

    fn update_list(s: &mut State, selection: usize) {
        let view = s.p.picker.list_view;
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let x = vse.scroll.x;
        let height = vse.layout(dse).content.height;

        let rows = height.max(1);
        let mut start = s.p.picker.offset.min(s.p.picker.filtered.len().saturating_sub(rows));
        if selection < start {
            start = selection;
        } else if selection >= start + rows {
            start = selection + 1 - rows;
        }
        let end = (start + height).min(s.p.picker.filtered.len());

        let text = s.p.picker.filtered[start..end]
            .iter()
            .map(|&index| s.p.picker.display_text[index].as_str())
            .collect::<Vec<_>>()
            .join("\n");

        s.p.picker.offset = start;

        Self::set_text(s, view, text);

        ViewProtocol::scroll_to(s, view, Pos::new(x, 0));
        ActionProtocol::move_cursor_to_pos(s, view, Pos::new(x, selection - start), true);
    }

    fn set_text(s: &mut State, view: ViewId, text: String) {
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get_mut(&doc).unwrap();

        let read_only = dse.read_only;

        // Temporarily set the mode to not read only for the modifications.
        dse.read_only = false;

        let len = dse.doc.data.len();

        ActionProtocol::remove(s, view, 0, len);
        ActionProtocol::insert(s, view, text);
        ActionProtocol::move_cursor_to_pos(s, view, Pos::default(), true);
        ActionProtocol::saved(s, doc);

        // Reset the read only flag after the modifications.
        s.doc_store.get_mut(&doc).unwrap().read_only = read_only;
    }
}
