use crate::state::{
    DirTypes, DocStoreTypes, State,
    ViewStoreTypes::{self, TabWidth},
};

pub fn setup(state: &mut State) {
    state.dir.doc = state.create_doc(None).unwrap();
    state.dir.view = state.create_view(state.dir.doc);
}

pub fn exec_doc_event(state: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != state.dir.doc {
        return;
    }

    let dse = state.doc_store.get_mut(&state.dir.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Dir;
    dse.read_only = true;
    dse.max_cursors = Some(1);
}

pub fn exec_view_event(state: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != state.dir.view {
        return;
    }

    let vse = state.view_store.get_mut(&state.dir.view).unwrap();
    vse.layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: true,
        mode_line: 1,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };
    vse.mode_line_config = DirTypes::DirModeLine::mode_line();
}
