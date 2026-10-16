use crate::state::{
    DocStoreTypes, State,
    ViewStoreTypes::{self, TabWidth},
};

pub fn setup(s: &mut State) {
    s.p.mp.doc = s.create_doc(None).unwrap();
    s.p.mp.view = s.create_view(s.p.mp.doc);
}

pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != s.p.mp.doc {
        return;
    }

    let dse = s.doc_store.get_mut(&s.p.mp.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Mp;
}

pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != s.p.mp.view {
        return;
    }

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
