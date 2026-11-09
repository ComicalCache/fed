use std::path::PathBuf;

#[derive(Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub dir: bool,
}
