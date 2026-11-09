use crate::protocols::dir::DirEntryKind;
use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct DirHeaderLayer {}

impl Layer for DirHeaderLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, _: ViewStoreTypes::Scroll,
        _: &[VisualOffsetMapping], _: &ViewStoreEntry, _: &DocStoreEntry,
    ) {
        if let Some(entry) = s.p.dir.entries.get(y)
            && matches!(entry.kind, DirEntryKind::Header)
        {
            for cell in row {
                cell.face.merge(s.theme.dir_header);
            }
        }
    }
}
