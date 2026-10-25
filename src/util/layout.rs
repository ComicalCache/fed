use crate::state::DocStoreEntry;
use crate::state::ViewStoreEntry;
use crate::types::render::VisualOffsetMapping;
use crate::types::render::{self};

pub fn vom(
    line: usize, vse: &ViewStoreEntry, dse: &DocStoreEntry,
) -> (Vec<VisualOffsetMapping>, usize) {
    let start = dse.doc.data.line_start_offset(line);
    let end = dse.doc.data.line_end_offset(line);
    let line = dse.doc.data.slice(start..end);

    let mut decs = Vec::new();
    dse.decs.range(start, end, &mut decs);
    vse.decs.range(start, end, &mut decs);

    render::layout_vom(&line, start, usize::MAX, &decs, &vse.layout)
}
