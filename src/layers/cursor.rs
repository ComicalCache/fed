use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct CursorLayer {}

impl Layer for CursorLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], _: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        // This is kind of a hack: ideally I don't have to compare the memory
        // addresses of the ViewStoreEntrys.
        let active = s
            .active_views()
            .into_flat_iter()
            .filter_map(|id| s.view_store.get(id))
            .any(|active| std::ptr::eq(vse, active));

        let face = if active { s.theme.active_cursor } else { s.theme.cursor };

        for (x, cell) in row.iter_mut().enumerate() {
            let visual_x = scroll.x + x;

            let Some(vo) = VisualOffsetMapping::at(voms, visual_x) else {
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
