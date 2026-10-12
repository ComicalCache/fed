use std::path::PathBuf;

use crate::{
    state::DirTypes::{Entry, PrintableEntry},
    types::render::WindowId,
};

#[allow(unused)]
pub enum DirCmd {
    Init,
    InitCompletion { entries: Vec<Entry>, printables: Vec<PrintableEntry> },
    ReplaceWindow { window: WindowId },

    Create { path: PathBuf, dir: bool },
    Rename { old: PathBuf, new: PathBuf },
    Delete { path: PathBuf, recursive: bool },

    Select,
}
