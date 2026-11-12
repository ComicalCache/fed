use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewStoreTypes;
use crate::state::ViewStoreTypes::TabWidth;
use crate::types::Face;
use crate::types::render::ModeLineConfig;
use crate::types::render::ModeLineWidget;

pub fn setup(s: &mut State) {
    s.p.grep.doc = s.create_doc(None).unwrap();
    let dse = s.doc_store.get_mut(&s.p.grep.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Grep;
    dse.read_only = true;
    dse.max_cursors = Some(1);

    s.p.grep.view = s.create_view(s.p.grep.doc);
    let vse = s.view_store.get_mut(&s.p.grep.view).unwrap();
    vse.layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: true,
        mode_line: 1,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };
    vse.mode_line_config = ModeLineConfig {
        left: vec![ModeLineWidget::Text { text: "[GREP]".to_string(), face: Face::default() }],
        right: vec![ModeLineWidget::SearchMatches { face: Face::default() }],
    };
}
