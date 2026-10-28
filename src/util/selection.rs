use unicode_segmentation::UnicodeSegmentation;

use crate::state::DocStoreEntry;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::Cursor;

pub fn lines(cursor: &Cursor, dse: &DocStoreEntry) -> (usize, usize) {
    let anchor = dse.doc.data.line_idx_at_offset(cursor.anchor);
    let offset = dse.doc.data.line_idx_at_offset(cursor.offset);

    (anchor.min(offset), anchor.max(offset))
}

pub fn has_last_line(vse: &ViewStoreEntry, dse: &DocStoreEntry) -> bool {
    vse.cursors.list.iter().any(|c| lines(c, dse).1 == dse.doc.data.line_count() - 1)
}

pub fn offsets(vse: &ViewStoreEntry, dse: &DocStoreEntry) -> Vec<(usize, usize)> {
    let linewise = vse.mode() == ViewStoreTypes::Mode::VisualLine;

    let mut offsets = Vec::with_capacity(vse.cursors.list.len());
    for cursor in &vse.cursors.list {
        if linewise {
            let (first, last) = lines(cursor, dse);
            offsets
                .push((dse.doc.data.line_start_offset(first), dse.doc.data.line_end_offset(last)));
        } else {
            let start = cursor.anchor.min(cursor.offset);
            let mut end = cursor.anchor.max(cursor.offset);

            let text = dse.doc.data.slice(end..dse.doc.data.len());
            if let Some(grapheme) = text.graphemes(true).next() {
                end += grapheme.len();
            }

            offsets.push((start, end));
        }
    }

    if !linewise {
        return offsets;
    }

    offsets.sort_unstable();

    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(offsets.len());
    for (start, end) in offsets {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }

    merged
}
