use crate::{
    render::{ModeLineConfig, ModeLineWidget},
    types::Face,
};

pub struct ReadModeLine {}

impl ReadModeLine {
    pub fn mode_line() -> ModeLineConfig {
        ModeLineConfig {
            left: vec![
                ModeLineWidget::DocMode { face: Face::default() },
                ModeLineWidget::ViewMode { face: Face::default() },
                ModeLineWidget::ReadWrite { face: Face::default() },
            ],
            right: vec![ModeLineWidget::CursorPos { face: Face::default() }],
        }
    }
}
