use piece_table::Slice;

use crate::{
    fed::FEvent,
    protocols::{
        action::ActionProtocol,
        mp::MpKind,
        picker::{PCallback, PickerCmd, PickerEntry, PickerRenderer},
        view::ViewProtocol,
    },
    state::{State, ViewId, ViewStoreTypes},
    types::{Motion, Pos, Rect, render::ZLayer},
};

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

        s.p.picker.entries = entries;
        s.p.picker.pcallback = Some(callback);

        Self::set_text(s, s.p.picker.input_view, String::new());

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
        s.p.picker.filtered.clear();

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

        let mut filtered = Vec::new();
        let mut text = String::new();
        for (index, entry) in s.p.picker.entries.iter().enumerate() {
            let label = entry.to_string().replace('\r', "\\r").replace('\n', "\\n");
            if label.to_lowercase().contains(&query) {
                if !filtered.is_empty() {
                    text.push('\n');
                }

                text.push_str(&label);
                filtered.push(index);
            }
        }

        s.p.picker.filtered = filtered;

        Self::set_text(s, s.p.picker.list_view, text);
    }

    pub fn change_selection(s: &mut State, motion: Motion) {
        if s.p.picker.window.is_none() || s.p.picker.filtered.is_empty() {
            return;
        }

        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.list_view)
                .unwrap();

        let x = vse.scroll.x;
        let y = dse.doc.data.get_line_of_byte(vse.cursors.list[0].offset);

        let last = s.p.picker.filtered.len() - 1;
        let y = match motion {
            Motion::Up => y.saturating_sub(1).min(last),
            Motion::Down => y.saturating_add(1).min(last),
            _ => return,
        };

        ActionProtocol::move_cursor_to_pos(s, s.p.picker.list_view, Pos::new(x, y), true);
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

        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.picker.list_view)
                .unwrap();

        let y = dse.doc.data.get_line_of_byte(vse.cursors.list[0].offset);
        let Some(&index) = s.p.picker.filtered.get(y) else { return };

        let selected = s.p.picker.entries.remove(index);
        Self::close(s, Some(selected));
    }

    pub fn resize(s: &mut State, width: usize, height: usize) {
        let Some(window) = s.p.picker.window else { return };

        let w = s.p.picker.pref_width.max(3).min(width);
        let h = s.p.picker.pref_height.max(5).min(height);

        s.workspace.reposition_floating(
            window,
            Pos::new(width.saturating_sub(w) / 2, height.saturating_sub(h) / 10),
        );
        s.workspace.resize_floating(window, w, h);

        let (input, list) = if w >= 3 && h >= 5 {
            (Rect::new(Pos::new(1, 1), w - 2, 1), Rect::new(Pos::new(1, 3), w - 2, h - 4))
        } else {
            (Rect::default(), Rect::default())
        };

        s.view_store.get_mut(&s.p.picker.input_view).unwrap().rect = input;
        s.view_store.get_mut(&s.p.picker.list_view).unwrap().rect = list;

        for view in [s.p.picker.input_view, s.p.picker.list_view] {
            ViewProtocol::scroll_if_needed(s, view);
            let _ = s.fevent_tx.send(FEvent::View(ViewStoreTypes::Event::Resized { id: view }));
        }
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
