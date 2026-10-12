use crate::{
    state::{DirTypes, DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes},
    types::{
        Face,
        render::{Cell, Layer, VisualOffsetMapping},
    },
};

pub struct DirLayer {}

impl Layer for DirLayer {
    fn apply(
        &self, state: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], _: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        if let Some(entry) = state.dir.entries.get(y) {
            let path = match entry.kind {
                DirTypes::EntryKind::File => Face::default(),
                DirTypes::EntryKind::Dir => {
                    Face { fg: state.theme.selection.bg, ..Face::default() }
                }
                DirTypes::EntryKind::Symlink => {
                    Face { fg: state.theme.gutter.fg, ..Face::default() }
                }
                DirTypes::EntryKind::Header => {
                    for cell in row.iter_mut() {
                        cell.face.merge(state.theme.dir_header);
                    }

                    return;
                }
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
    }
}
