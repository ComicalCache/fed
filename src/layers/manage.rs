use crate::{
    state::{DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
    types::render::{Cell, Layer, VisualOffsetMapping},
};

pub struct CurrLineLayer {}

impl Layer for CurrLineLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], y: usize, _scroll: ViewStoreTypes::Scroll,
        _: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let cursor_y = dse.doc.data.get_line_of_byte(offset);

        if cursor_y != y {
            return;
        }

        for cell in row {
            cell.face.underline = Some(true);
            cell.face.uc = state.theme.selection.bg;
        }
    }
}
