use crate::{
    state::{DocStoreTypes, State, ViewStoreTypes},
    types::{
        Face,
        render::{ModeLineConfig, ModeLineWidget},
    },
};

pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::ModeChanged { id, mode: DocStoreTypes::Mode::Read } = event else {
        return;
    };

    let dse = s.doc_store.get_mut(id).unwrap();
    dse.read_only = true;

    let views = s.index.doc_to_views(*id).cloned().unwrap_or_default();
    for view in views {
        let vse = s.view_store.get_mut(&view).unwrap();

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

    let doc = s.index.view_to_doc(*id).unwrap();
    let dse = s.doc_store.get(&doc).unwrap();

    if dse.mode != DocStoreTypes::Mode::Read {
        return;
    }

    let vse = s.view_store.get_mut(id).unwrap();

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
