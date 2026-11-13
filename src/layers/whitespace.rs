use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct TrailingWhitespaceLayer {}

impl Layer for TrailingWhitespaceLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        let Some(line) = vse.view_cache.lines.get(y.saturating_sub(scroll.y)) else { return };
        let line = line.strip_suffix('\n').unwrap_or(line);
        let line_start = dse.doc.data.line_start_offset(y);

        let start = line_start + line.trim_end_matches(char::is_whitespace).len();
        let end = line_start + line.len();
        if start == end {
            return;
        }

        let cursor = vse
            .cursors
            .list
            .iter()
            .filter(|c| c.offset >= line_start && c.offset <= end)
            .map(|c| c.offset)
            .max();

        for mapping in voms {
            if mapping.byte_start == mapping.byte_end
                || mapping.byte_start < start
                || mapping.byte_end > end
                || cursor.is_some_and(|offset| mapping.byte_start <= offset)
            {
                continue;
            }

            let start = mapping.visual_x.saturating_sub(scroll.x).min(row.len());
            let end = mapping
                .visual_x
                .saturating_add(mapping.visual_width)
                .saturating_sub(scroll.x)
                .min(row.len());
            for cell in &mut row[start..end] {
                cell.face.merge(s.theme.error_whitespace);
            }
        }
    }
}
