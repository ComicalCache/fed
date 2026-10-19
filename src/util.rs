pub mod ansi;
pub mod format;
pub mod layout;
mod macros;
pub mod motion;
pub mod mp;
pub mod path;
pub mod text;

pub(crate) use macros::newtype;

use crate::{
    state::{DocStoreEntry, ViewStoreEntry},
    types::Pos,
    util,
};

pub fn pos_to_offset(pos: Pos, vse: &ViewStoreEntry, dse: &DocStoreEntry) -> usize {
    let y = pos.y.min(dse.doc.data.lines().saturating_sub(1));
    let start = dse.doc.data.get_line_start_byte(y);

    let (vom, _) = util::layout::vom(y, vse, dse);

    vom.iter().rev().find(|vo| vo.visual_x <= pos.x).map_or(start, |vo| vo.offset)
}

pub fn offset_to_pos(offset: usize, vse: &ViewStoreEntry, dse: &DocStoreEntry) -> Pos {
    let y = dse.doc.data.get_line_of_byte(offset);

    let (vom, _) = util::layout::vom(y, vse, dse);
    let x = vom
        .iter()
        .find(|vo| vo.offset >= offset)
        .map_or_else(|| vom.last().map_or(0, |vo| vo.visual_x), |vo| vo.visual_x);

    Pos::new(x, y)
}
