use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Face;
use crate::types::render::ModeLineConfig;
use crate::types::render::ModeLineWidget;

pub fn dmode(s: &mut State, doc: DocId) {
    s.doc_store.get_mut(&doc).unwrap().read_only = true;

    let views = s.index.doc_to_views(doc).cloned().unwrap_or_default();
    for view in views {
        vinit(s, view);
    }
}

pub fn vinit(s: &mut State, view: ViewId) {
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
