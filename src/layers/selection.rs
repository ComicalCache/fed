use crate::{
    state::{DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
    types::render::{Cell, Layer, VisualOffsetMapping},
};

pub struct SelectionLayer {}

impl Layer for SelectionLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], _: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        for (x, cell) in row.iter_mut().enumerate() {
            let visual_x = scroll.x + x;

            let Some(vo) = VisualOffsetMapping::at(voms, visual_x) else {
                continue;
            };

            for cursor in &vse.cursors.list {
                let start = cursor.offset.min(cursor.anchor);
                let end = cursor.offset.max(cursor.anchor);

                if start <= vo.offset && vo.offset <= end {
                    cell.face.merge(s.theme.selection);
                    break;
                }
            }
        }
    }
}
