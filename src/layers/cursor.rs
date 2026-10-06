use crate::{
    render::{Cell, Layer, VisualOffsetMapping},
    state::{DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
};

pub struct CursorLayer {}

impl Layer for CursorLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], _: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        for (x, cell) in row.iter_mut().enumerate() {
            let visual_x = scroll.x + x;

            let Some(vo) =
                voms.iter().find(|vo| visual_x >= vo.visual_x && visual_x < vo.visual_x + vo.width)
            else {
                continue;
            };

            for cursor in &vse.cursors.list {
                if cursor.offset == vo.offset {
                    cell.face.merge(state.theme.cursor);
                    break;
                }
            }
        }
    }
}
