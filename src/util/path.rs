use std::path::{Path, PathBuf};

pub fn normalize<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    path.canonicalize().unwrap_or_else(|_| path.absolute().unwrap_or_else(|_| path.to_path_buf()))
}
