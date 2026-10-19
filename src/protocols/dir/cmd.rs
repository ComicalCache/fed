use std::path::PathBuf;

use crate::{
    protocols::dir::{DirEntry, PrintableDirEntry},
    types::render::WindowId,
};

#[allow(unused)]
pub enum DirCmd {
    Init,
    InitCompletion {
        entries: Vec<DirEntry>,
        printables: Vec<PrintableDirEntry>,
    },
    ReplaceWindow {
        window: WindowId,
        path: Option<PathBuf>,
    },

    Create {
        path: PathBuf,
        dir: bool,
    },
    Rename {
        old: PathBuf,
        new: PathBuf,
    },
    Delete {
        path: PathBuf,
        recursive: bool,
    },

    Select,
    SelectCompletion {
        revision: usize,
        window: WindowId,
        path: PathBuf,
        res: Result<String, String>,
    },
}
