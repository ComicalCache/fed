use crate::{
    state::{DocStoreTypes, State, ViewStoreTypes},
    types::{
        Face,
        render::{ModeLineConfig, ModeLineWidget},
    },
    util::debug_panic,
};

pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::ModeChanged { id, mode: DocStoreTypes::Mode::Read } = event else {
        return;
    };

    let dse = s.doc_store.get_mut(id).unwrap();
    dse.read_only = true;

    let views: Vec<_> = s.index.doc_to_views(*id).map_or_default(|v| v.iter().copied().collect());
    for view in views {
        let Some(vse) = s.view_store.get_mut(&view) else {
            debug_panic!();
            continue;
        };

        vse.layout.tab_width = ViewStoreTypes::TabWidth::default();
        vse.layout.gutter = false;
        vse.layout.replacements = ViewStoreTypes::Replacements::none();
        vse.layout.rulers.clear();
        vse.mode_line_config = ModeLineConfig {
            left: vec![
                ModeLineWidget::DocMode { face: Face::default() },
                ModeLineWidget::ViewMode { face: Face::default() },
                ModeLineWidget::ReadWrite { face: Face::default() },
            ],
            right: vec![ModeLineWidget::CursorPos { face: Face::default() }],
        };
    }
}

pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else { return };

    let Some(doc) = s.index.view_to_doc(*id) else {
        debug_panic!();
        return;
    };
    let Some(dse) = s.doc_store.get(&doc) else {
        debug_panic!();
        return;
    };

    if dse.mode != DocStoreTypes::Mode::Read {
        return;
    }

    let Some(vse) = s.view_store.get_mut(id) else {
        debug_panic!();
        return;
    };

    vse.layout.tab_width = ViewStoreTypes::TabWidth::default();
    vse.layout.gutter = false;
    vse.layout.replacements = ViewStoreTypes::Replacements::none();
    vse.layout.rulers.clear();
    vse.mode_line_config = ModeLineConfig {
        left: vec![
            ModeLineWidget::DocMode { face: Face::default() },
            ModeLineWidget::ViewMode { face: Face::default() },
            ModeLineWidget::ReadWrite { face: Face::default() },
        ],
        right: vec![ModeLineWidget::CursorPos { face: Face::default() }],
    };
}
