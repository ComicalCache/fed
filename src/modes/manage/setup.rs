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
    s.p.manage.doc = s.create_doc(None).unwrap();
    s.p.manage.view = s.create_view(s.p.manage.doc);
}

pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
    let DocStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != s.p.manage.doc {
        return;
    }

    let dse = s.doc_store.get_mut(&s.p.manage.doc).unwrap();
    dse.mode = DocStoreTypes::Mode::Manage;
    dse.read_only = true;
    dse.max_cursors = Some(1);
}

pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
    let ViewStoreTypes::Event::Created { id } = event else {
        return;
    };
    if *id != s.p.manage.view {
        return;
    }

    let vse = s.view_store.get_mut(&s.p.manage.view).unwrap();
    vse.layout = ViewStoreTypes::Layout {
        tab_width: TabWidth::default(),
        gutter: true,
        mode_line: 1,
        replacements: ViewStoreTypes::Replacements::none(),
        rulers: Vec::new(),
    };
    vse.mode_line_config = ModeLineConfig {
        left: vec![ModeLineWidget::Text { text: "[MANAGE]".to_string(), face: Face::default() }],
        right: vec![],
    };
}
