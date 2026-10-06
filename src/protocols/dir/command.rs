use std::path::PathBuf;

use crate::render::WindowId;

pub enum DirCommand {
    Init,
    ReplaceWindow { window: WindowId },
    Create { path: PathBuf, dir: bool },
    Rename { old: PathBuf, new: PathBuf },
    Delete { path: PathBuf, recursive: bool },
    Select,
}
