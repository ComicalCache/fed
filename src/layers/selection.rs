use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;
use crate::util::selection;

pub struct SelectionLayer {}

impl Layer for SelectionLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        if vse.mode() == ViewStoreTypes::Mode::VisualLine {
            if y >= dse.doc.data.line_count()
                || !vse.cursors.list.iter().any(|c| {
                    let (first, last) = selection::lines(c, dse);
                    first <= y && y <= last
                })
            {
                return;
            }

            let mut end = dse.doc.data.line_end_offset(y);
            if y + 1 < dse.doc.data.line_count() && vse.layout.replacements.newline.is_none() {
                end -= 1;
            }

            for (x, cell) in row.iter_mut().enumerate() {
                if let Some(vo) = VisualOffsetMapping::at(voms, scroll.x + x)
                    && vo.offset < end
                {
                    cell.face.merge(s.theme.selection);
                }
            }

            return;
        }

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
