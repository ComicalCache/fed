use crate::{
    render::{ModeLineConfig, ModeLineWidget},
    types::Face,
};

pub struct DirModeLine {}

impl DirModeLine {
    pub fn mode_line() -> ModeLineConfig {
        ModeLineConfig {
            left: vec![
                ModeLineWidget::Text { text: "[DIR]".to_string(), face: Face::default() },
                ModeLineWidget::Custom(|state, _, _| {
                    (state.dir.pwd.display().to_string(), Face::default())
                }),
            ],
            right: vec![ModeLineWidget::Custom(|state, vse, dse| {
                let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
                let y = dse.doc.data.get_line_of_byte(offset);

                (state.dir.entries[y].kind.to_string(), Face::default())
            })],
        }
    }
}
