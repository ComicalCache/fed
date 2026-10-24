use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewStoreTypes::TabWidth;
use crate::state::ViewStoreTypes::{self};

pub fn setup(s: &mut State) {
    s.p.picker.input_doc = s.create_doc(None).unwrap();
    let input_dse = s.doc_store.get_mut(&s.p.picker.input_doc).unwrap();
    input_dse.mode = DocStoreTypes::Mode::Picker;
    input_dse.max_cursors = Some(1);

    s.p.picker.list_doc = s.create_doc(None).unwrap();
    let list_dse = s.doc_store.get_mut(&s.p.picker.list_doc).unwrap();
    list_dse.mode = DocStoreTypes::Mode::Picker;
    list_dse.read_only = true;
    list_dse.max_cursors = Some(1);

    let layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: false,
        mode_line: 0,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };

    s.p.picker.input_view = s.create_view(s.p.picker.input_doc);
    s.view_store.get_mut(&s.p.picker.input_view).unwrap().layout = layout.clone();

    s.p.picker.list_view = s.create_view(s.p.picker.list_doc);
    s.view_store.get_mut(&s.p.picker.list_view).unwrap().layout = layout;
}
