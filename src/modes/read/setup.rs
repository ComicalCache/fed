use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Face;
use crate::types::render::ModeLineConfig;
use crate::types::render::ModeLineWidget;

pub fn dmode(s: &mut State, doc: DocId) { s.doc_store.get_mut(&doc).unwrap().read_only = true; }

pub fn vmode(s: &mut State, view: ViewId) {
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
        right: vec![
            ModeLineWidget::SearchMatches { face: Face::default() },
            ModeLineWidget::CursorPos { face: Face::default() },
        ],
    };
}
