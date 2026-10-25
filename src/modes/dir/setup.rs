use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewStoreTypes::TabWidth;
use crate::state::ViewStoreTypes::{self};
use crate::types::Face;
use crate::types::render::ModeLineConfig;
use crate::types::render::ModeLineWidget;
use crate::util;

pub fn setup(s: &mut State) {
    s.p.dir.pwd = std::env::current_dir().map_or_default(util::path::normalize);
    s.p.dir.doc = s.create_doc(None).unwrap();
    let dse = s.doc_store.get_mut(&s.p.dir.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Dir;
    dse.read_only = true;
    dse.max_cursors = Some(1);

    s.p.dir.view = s.create_view(s.p.dir.doc);
    let vse = s.view_store.get_mut(&s.p.dir.view).unwrap();
    vse.layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: true,
        mode_line: 1,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };
    vse.mode_line_config = ModeLineConfig {
        left: vec![
            ModeLineWidget::Text { text: "[DIR]".to_string(), face: Face::default() },
            ModeLineWidget::Custom(|s, _, _| (s.p.dir.pwd.display().to_string(), Face::default())),
        ],
        right: vec![ModeLineWidget::Custom(|s, vse, dse| {
            let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
            let y = dse.doc.data.line_idx_at_offset(offset);

            (s.p.dir.entries.get(y).map_or_default(|e| e.kind.to_string()), Face::default())
        })],
    };
}
