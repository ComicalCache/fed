use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct SearchLayer {}

impl Layer for SearchLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], _: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        let Some(search) = &vse.search else { return };

        for mapping in voms {
            if mapping.byte_start == mapping.byte_end {
                continue;
            }

            let start = mapping.visual_x.saturating_sub(scroll.x).min(row.len());
            let end = mapping
                .visual_x
                .saturating_add(mapping.visual_width)
                .saturating_sub(scroll.x)
                .min(row.len());

            if start == end {
                continue;
            }

            let idx = search.matches.partition_point(|m| {
                m.end < mapping.byte_start || m.end == mapping.byte_start && m.start != m.end
            });
            if search.matches.get(idx).is_some_and(|m| m.start < mapping.byte_end) {
                for cell in &mut row[start..end] {
                    cell.face.merge(s.theme.search_match);
                }
            }
        }
    }
}
