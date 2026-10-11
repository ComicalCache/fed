use std::path::PathBuf;

use crate::{
    render::WindowId,
    state::DirTypes::{Entry, PrintableEntry},
};

pub enum DirCmd {
    Init,
    InitCompletion { entries: Vec<Entry>, printables: Vec<PrintableEntry> },
    ReplaceWindow { window: WindowId },
    Create { path: PathBuf, dir: bool },
    Rename { old: PathBuf, new: PathBuf },
    Delete { path: PathBuf, recursive: bool },
    Select,
}
