use crate::state::{
    DocStoreTypes, State,
    ViewStoreTypes::{self, TabWidth},
};

pub fn setup(state: &mut State) {
    state.mp.doc = state.create_doc(None).unwrap();
    state.mp.view = state.create_view(state.mp.doc);
}

pub fn exec_doc_event(state: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != state.mp.doc {
        return;
    }

    let dse = state.doc_store.get_mut(&state.mp.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Mp;
}

pub fn exec_view_event(state: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != state.mp.view {
        return;
    }

    let vse = state.view_store.get_mut(&state.mp.view).unwrap();
    vse.layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: false,
        mode_line: 0,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };
    vse.cursors.list.clear();
}
