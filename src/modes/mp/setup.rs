use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewStoreTypes::TabWidth;
use crate::state::ViewStoreTypes::{self};

pub fn setup(s: &mut State) {
    s.p.mp.doc = s.create_doc(None).unwrap();
    let dse = s.doc_store.get_mut(&s.p.mp.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Mp;
    dse.max_cursors = Some(1);

    s.p.mp.view = s.create_view(s.p.mp.doc);
    let vse = s.view_store.get_mut(&s.p.mp.view).unwrap();
    vse.layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: false,
        mode_line: 0,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };
    vse.cursors.list.clear();
}
