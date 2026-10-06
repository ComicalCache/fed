use crate::{
    render::{Cell, Layer, VisualOffsetMapping},
    state::{DirTypes, DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
    types::Face,
};

pub struct DirLayer {}

impl Layer for DirLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        if let Some(entry) = state.dir.entries.get(y) {
            let path = match entry.kind {
                DirTypes::EntryKind::Dir => {
                    Face { fg: state.theme.selection.bg, ..Face::default() }
                }
                DirTypes::EntryKind::Symlink => state.theme.dir_symlink,
                DirTypes::EntryKind::File => Face::default(),
            };
            let metadata = Face { fg: state.theme.gutter.fg, ..Face::default() };

            let line_start = dse.doc.data.get_line_start_byte(y);
            let offset = line_start + entry.path_offset;

            for (x, cell) in row.iter_mut().enumerate() {
                let visual_x = scroll.x + x;
                if let Some(vo) = voms
                    .iter()
                    .find(|vo| visual_x >= vo.visual_x && visual_x < vo.visual_x + vo.width)
                {
                    if vo.offset < offset {
                        cell.face.merge(metadata);
                    } else if vo.offset >= offset {
                        cell.face.merge(path);
                    }
                }
            }
        }

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let cursor_y = dse.doc.data.get_line_of_byte(offset);

        if cursor_y != y {
            return;
        }

        for cell in row {
            cell.face.underline = Some(true);
            cell.face.uc = state.theme.selection.bg;
        }
    }
}
