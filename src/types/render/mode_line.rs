use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::types::Face;
use crate::util;

pub enum ModeLineWidget {
    DocMode { face: Face },
    ViewMode { face: Face },
    ReadWrite { face: Face },
    FilePath { face: Face },
    CursorPos { face: Face },
    Text { text: String, face: Face },
    Custom(fn(&State, &ViewStoreEntry, &DocStoreEntry) -> (String, Face)),
}

impl ModeLineWidget {
    pub fn render(&self, s: &State, vse: &ViewStoreEntry, dse: &DocStoreEntry) -> (String, Face) {
        match self {
            Self::DocMode { face } => (format!("[{}]", dse.mode.to_string().to_uppercase()), *face),
            Self::ViewMode { face } => {
                (format!("[{}]", vse.mode().to_string().to_uppercase()), *face)
            }
            Self::ReadWrite { face } => {
                (if dse.read_only { "[R]".to_string() } else { "[RW]".to_string() }, *face)
            }
            Self::FilePath { face } => {
                dse.doc.path.as_ref().map(|p| p.display().to_string()).map_or_else(
                    || (format!("[SCRATCHPAD{}]", if dse.doc.modified { "*" } else { "" }), *face),
                    |path| (format!("[{path}{}]", if dse.doc.modified { "*" } else { "" }), *face),
                )
            }
            Self::CursorPos { face } => {
                let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
                let y = dse.doc.data.get_line_of_byte(offset);

                let (vom, _) = util::layout::vom(y, vse, dse);
                let x = vom.iter().find(|vo| vo.offset >= offset).map_or(0, |vo| vo.visual_x);

                (format!("[{}:{} {}]", y + 1, x + 1, vse.cursors.list.len()), *face)
            }
            Self::Text { text, face } => (text.clone(), *face),
            Self::Custom(f) => f(s, vse, dse),
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
                ModeLineWidget::DocMode { face: Face::default() },
                ModeLineWidget::ViewMode { face: Face::default() },
                ModeLineWidget::ReadWrite { face: Face::default() },
                ModeLineWidget::FilePath { face: Face::default() },
            ],
            right: vec![ModeLineWidget::CursorPos { face: Face::default() }],
        }
    }
}
