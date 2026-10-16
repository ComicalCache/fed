use crate::{
    state::{DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
    types::render::{Cell, Layer, VisualOffsetMapping},
};

pub struct CursorLayer {}

impl Layer for CursorLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], _: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        // This is kind of a hack: ideally I don't have to compare the memory
        // addresses of the ViewStoreEntrys.
        let active = s
            .active_view()
            .and_then(|id| s.view_store.get(&id))
            .is_some_and(|active| std::ptr::eq(vse, active));

        let face = if active { s.theme.active_cursor } else { s.theme.cursor };

        for (x, cell) in row.iter_mut().enumerate() {
            let visual_x = scroll.x + x;

            let Some(vo) =
                voms.iter().find(|vo| visual_x >= vo.visual_x && visual_x < vo.visual_x + vo.width)
            else {
                continue;
            };

            for cursor in &vse.cursors.list {
                if cursor.offset == vo.offset {
                    cell.face.merge(face);
                    break;
                }
            }
        }
    }
}
