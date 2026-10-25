use crate::protocols::dir::DirEntryKind;
use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;
use crate::types::Face;
use crate::types::render::Cell;
use crate::types::render::Layer;
use crate::types::render::VisualOffsetMapping;

pub struct DirLayer {}

impl Layer for DirLayer {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], _: &ViewStoreEntry, dse: &DocStoreEntry,
    ) {
        if let Some(entry) = s.p.dir.entries.get(y) {
            let path = match entry.kind {
                DirEntryKind::File => Face::default(),
                DirEntryKind::Dir => Face { fg: s.theme.selection.bg, ..Face::default() },
                DirEntryKind::Symlink => Face { fg: s.theme.gutter.fg, ..Face::default() },
                DirEntryKind::Header => {
                    for cell in row.iter_mut() {
                        cell.face.merge(s.theme.dir_header);
                    }

                    return;
                }
            };
            let metadata = Face { fg: s.theme.gutter.fg, ..Face::default() };

            let line_start = dse.doc.data.line_start_offset(y);
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
