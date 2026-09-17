use crate::{
    state::{DocumentStoreEntry, ViewStoreEntry},
    types::Face,
};

pub enum ModeLineWidget {
    Mode { face: Face },
    FilePath { face: Face },
    CursorPos { face: Face },
    Text { text: String, face: Face },
    Custom(fn(&ViewStoreEntry, &DocumentStoreEntry) -> Vec<(String, Face)>),
}

impl ModeLineWidget {
    pub fn render(&self, vse: &ViewStoreEntry, dse: &DocumentStoreEntry) -> Vec<(String, Face)> {
        match self {
            ModeLineWidget::Mode { face } => {
                vec![(format!("[{}]", vse.mode.to_string().to_uppercase()), *face)]
            }
            ModeLineWidget::FilePath { face } => {
                match dse.doc.path.as_ref().map(|p| p.display().to_string()) {
                    Some(path) => vec![(
                        format!("[{path}{}]", if dse.doc.modified { "*" } else { "" }),
                        *face,
                    )],
                    None => vec![(
                        format!("[SCRATCHPAD{}]", if dse.doc.modified { "*" } else { "" }),
                        *face,
                    )],
                }
            }
            ModeLineWidget::CursorPos { face } => {
                let cursor = vse.cursors.list.first().map(|c| c.pos).unwrap_or_default();
                vec![(
                    format!("[{}:{} {}]", cursor.y + 1, cursor.x + 1, vse.cursors.list.len()),
                    *face,
                )]
            }
            ModeLineWidget::Text { text, face } => vec![(text.clone(), *face)],
            ModeLineWidget::Custom(f) => f(vse, dse),
        }
    }
}

pub struct ModeLineConfig {
    pub left: Vec<ModeLineWidget>,
    pub right: Vec<ModeLineWidget>,
}

impl Default for ModeLineConfig {
    fn default() -> Self {
        Self {
            left: vec![
                ModeLineWidget::Mode { face: Face::default() },
                ModeLineWidget::FilePath { face: Face::default() },
            ],
            right: vec![ModeLineWidget::CursorPos { face: Face::default() }],
        }
    }
}
