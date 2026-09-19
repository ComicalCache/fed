use piece_table::Slice;

use crate::{
    render,
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
                let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);

                let lines = dse.doc.data.lines();
                let mut y = lines.saturating_sub(1);
                for idx in 0..lines {
                    if offset >= dse.doc.data.get_line_start_byte(idx)
                        && (offset < dse.doc.data.get_line_end_byte(idx) || idx == lines - 1)
                    {
                        y = idx;
                        break;
                    }
                }

                let start = dse.doc.data.get_line_start_byte(y);
                let end = dse.doc.data.get_line_end_byte(y);
                let line = dse.doc.data.slice(start..end);

                let mut decs = Vec::new();
                dse.decs.range(start, end, &mut decs);
                vse.decs.range(start, end, &mut decs);

                let (vom, _) = render::layout_vom(&line, start, vse.tab_width, &decs);
                let x =
                    vom.iter().find(|vo| vo.offset >= offset).map(|vo| vo.visual_x).unwrap_or(0);

                vec![(format!("[{}:{} {}]", y + 1, x + 1, vse.cursors.list.len()), *face)]
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
