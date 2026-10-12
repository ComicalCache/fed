use crate::{
    debug_panic::debug_panic,
    state::{DocStoreTypes, ReadTypes, State, ViewStoreTypes},
};

pub fn exec_doc_event(state: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::ModeChanged { id, mode: DocStoreTypes::Mode::Read } = event else {
        return;
    };

    let dse = state.doc_store.get_mut(&id).unwrap();
    dse.read_only = true;

    let views: Vec<_> =
        state.index.doc_to_views(*id).map(|v| v.iter().copied().collect()).unwrap_or_default();
    for view in views {
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            continue;
        };

        vse.layout.tab_width = ViewStoreTypes::TabWidth::default();
        vse.layout.gutter = false;
        vse.layout.replacements = ViewStoreTypes::Replacements::none();
        vse.layout.rulers.clear();
        vse.mode_line_config = ReadTypes::ReadModeLine::mode_line();
    }
}

pub fn exec_view_event(state: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else { return };

    let Some(doc) = state.index.view_to_doc(*id) else {
        debug_panic!();
        return;
    };
    let Some(dse) = state.doc_store.get(&doc) else {
        debug_panic!();
        return;
    };

    if dse.mode != DocStoreTypes::Mode::Read {
        return;
    }

    let Some(vse) = state.view_store.get_mut(id) else {
        debug_panic!();
        return;
    };

    vse.layout.tab_width = ViewStoreTypes::TabWidth::default();
    vse.layout.gutter = false;
    vse.layout.replacements = ViewStoreTypes::Replacements::none();
    vse.layout.rulers.clear();
    vse.mode_line_config = ReadTypes::ReadModeLine::mode_line();
}
