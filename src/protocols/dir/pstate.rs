use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::path::PathBuf;

use tokio::task::JoinHandle;

use crate::state::DocId;
use crate::state::ViewId;

#[derive(Debug, Default, Clone)]
pub enum DirEntryKind {
    #[default]
    File,
    Dir,
    Symlink,
    Header,
}

impl Display for DirEntryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{self:?}") }
}

#[derive(Default, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub kind: DirEntryKind,

    pub path_offset: usize,
}

#[derive(Default)]
pub struct DirPState {
    pub doc: DocId,
    pub view: ViewId,

    pub pwd: PathBuf,
    pub entries: Vec<DirEntry>,
    pub revision: u64,

    pub fetch_task: Option<JoinHandle<()>>,
}

pub struct PrintableDirEntry {
    pub perms: String,
    pub links: String,
    pub owner: String,
    pub group: String,
    pub size: String,
    pub modified: String,
    pub name: String,
}
