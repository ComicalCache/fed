use piece_table::Slice;

use crate::{
    state::{DocStoreEntry, ViewStoreEntry},
    types::render::{self, VisualOffsetMapping},
};

pub fn vom(
    line: usize, vse: &ViewStoreEntry, dse: &DocStoreEntry,
) -> (Vec<VisualOffsetMapping>, usize) {
    let start = dse.doc.data.get_line_start_byte(line);
    let end = dse.doc.data.get_line_end_byte(line);
    let line = dse.doc.data.slice(start..end);

    let mut decs = Vec::new();
    dse.decs.range(start, end, &mut decs);
    vse.decs.range(start, end, &mut decs);

    render::layout_vom(&line, start, usize::MAX, &decs, &vse.layout)
}
