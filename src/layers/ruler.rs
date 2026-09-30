use crate::{
    render::{Cell, Layer, VisualOffsetMapping},
    state::{DocStoreEntry, State, ViewStoreEntry},
};

pub struct RulerLayer {}

impl Layer for RulerLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], scroll_x: usize, _: &[VisualOffsetMapping],
        vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        if vse.layout.rulers.is_empty() {
            return;
        }

        for (x, cell) in row.iter_mut().enumerate() {
            if vse.layout.rulers.contains(&(scroll_x + x)) {
                cell.face.merge(state.theme.ruler);
            }
        }
    }
}
