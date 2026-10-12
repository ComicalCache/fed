use crate::types::{
    Face,
    render::{ModeLineConfig, ModeLineWidget},
};

pub struct ManageModeLine {}

impl ManageModeLine {
    pub fn mode_line() -> ModeLineConfig {
        ModeLineConfig {
            left: vec![ModeLineWidget::Text {
                text: "[MANAGE]".to_string(),
                face: Face::default(),
            }],
            right: vec![],
        }
    }
}
