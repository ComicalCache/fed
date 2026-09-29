mod motion;

pub use motion::{apply_motion, motion_offsets};
use piece_table::Slice;

use crate::{
    render::{self, VisualOffsetMapping},
    state::{DocStoreEntry, ViewStoreEntry},
    types::Pos,
    util,
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

    render::layout_vom(&line, start, &decs, &vse.layout.replacements, vse.tab_width)
}

pub fn pos_to_offset(pos: Pos, vse: &ViewStoreEntry, dse: &DocStoreEntry) -> usize {
    let y = pos.y.min(dse.doc.data.lines().saturating_sub(1));
    let start = dse.doc.data.get_line_start_byte(y);

    let (vom, _) = util::vom(y, vse, dse);

    vom.iter().rev().find(|vo| vo.visual_x <= pos.x).map(|vo| vo.offset).unwrap_or(start)
}

pub fn offset_to_pos(offset: usize, vse: &ViewStoreEntry, dse: &DocStoreEntry) -> Pos {
    let y = dse.doc.data.get_line_of_byte(offset);

    let (vom, _) = util::vom(y, vse, dse);
    let x = vom
        .iter()
        .find(|vo| vo.offset >= offset)
        .map(|vo| vo.visual_x)
        .unwrap_or_else(|| vom.last().map(|vo| vo.visual_x).unwrap_or(0));

    Pos::new(x, y)
}
