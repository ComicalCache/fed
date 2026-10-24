use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct RulerLayer {}

impl Layer for RulerLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], _: usize, scroll: ViewStoreTypes::Scroll,
        _: &[VisualOffsetMapping], vse: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        if vse.layout.rulers.is_empty() {
            return;
        }

        for (x, cell) in row.iter_mut().enumerate() {
            if vse.layout.rulers.contains(&(scroll.x + x)) {
                cell.face.merge(s.theme.ruler);
            }
        }
    }
}
