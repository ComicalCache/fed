use crate::{
    render::{Cell, Layer, VisualOffsetMapping},
    state::{DocStoreEntry, State, ViewStoreEntry},
};

pub struct SelectionLayer {}

impl Layer for SelectionLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], scroll_x: usize, vom: &[VisualOffsetMapping],
        vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        let mut selected_xs = Vec::new();
        for vo in vom {
            for cursor in &vse.cursors.list {
                let start = cursor.offset.min(cursor.anchor);
                let end = cursor.offset.max(cursor.anchor);

                if start <= vo.offset && vo.offset <= end {
                    selected_xs.push(vo.visual_x);
                }
            }
        }

        for (x, cell) in row.iter_mut().enumerate() {
            if selected_xs.contains(&(scroll_x + x)) {
                cell.face.merge(state.theme.selection);
            }
        }
    }
}
