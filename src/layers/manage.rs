use crate::{
    state::{DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
    types::render::{Cell, Layer, VisualOffsetMapping},
};

pub struct CurrLineLayer {}

impl Layer for CurrLineLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, _: ViewStoreTypes::Scroll,
        _: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
        let cursor_y = dse.doc.data.get_line_of_byte(offset);

        if cursor_y != y {
            return;
        }

        for cell in row {
            cell.face.underline = Some(true);
            cell.face.uc = s.theme.selection.bg;
        }
    }
}
