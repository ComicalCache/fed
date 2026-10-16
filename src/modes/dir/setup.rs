use crate::{
    state::{
        DocStoreTypes, State,
        ViewStoreTypes::{self, TabWidth},
    },
    types::{
        Face,
        render::{ModeLineConfig, ModeLineWidget},
    },
};

pub fn setup(s: &mut State) {
    s.p.dir.doc = s.create_doc(None).unwrap();
    s.p.dir.view = s.create_view(s.p.dir.doc);
}

pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != s.p.dir.doc {
        return;
    }

    let dse = s.doc_store.get_mut(&s.p.dir.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Dir;
    dse.read_only = true;
    dse.max_cursors = Some(1);
}

pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != s.p.dir.view {
        return;
    }

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
            let y = dse.doc.data.get_line_of_byte(offset);

            (s.p.dir.entries.get(y).map_or_default(|e| e.kind.to_string()), Face::default())
        })],
    };
}
