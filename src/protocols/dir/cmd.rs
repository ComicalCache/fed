use std::path::PathBuf;

use crate::protocols::dir::DirEntry;
use crate::protocols::dir::PrintableDirEntry;
use crate::types::render::WindowId;

#[allow(unused)]
pub enum DirCmd {
    Init,
    InitCompletion { entries: Vec<DirEntry>, printables: Vec<PrintableDirEntry>, revision: u64 },
    Show { window: WindowId, path: Option<PathBuf> },

    Create { path: PathBuf, dir: bool },
    Rename { old: PathBuf, new: PathBuf },
    Delete { path: PathBuf, recursive: bool },

    Select,
}
