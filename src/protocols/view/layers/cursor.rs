use crate::{
    render::{Cell, Layer, VisualOffsetMapping},
    state::{DocStoreEntry, State, ViewStoreEntry},
};

pub struct CursorLayer {}

impl Layer for CursorLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], scroll_x: usize, vom: &[VisualOffsetMapping],
        vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        let mut cursor_xs = Vec::new();
        for vo in vom {
            for cursor in &vse.cursors.list {
                if cursor.offset == vo.offset {
                    cursor_xs.push(vo.visual_x);
                }
            }
        }

        for (x, cell) in row.iter_mut().enumerate() {
            if cursor_xs.contains(&(scroll_x + x)) {
                cell.face.merge(state.theme.cursor);
            }
        }
    }
}
