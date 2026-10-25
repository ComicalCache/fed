use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct CurrLineLayer {}

impl Layer for CurrLineLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, _: ViewStoreTypes::Scroll,
        _: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let cursor_y = dse.doc.data.line_idx_at_offset(offset);

        if cursor_y != y {
            return;
        }

        for cell in row {
            cell.face.underline = Some(true);
            cell.face.uc = s.theme.selection.bg;
        }
    }
}
