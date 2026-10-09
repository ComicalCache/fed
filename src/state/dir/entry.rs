use std::{
    fmt::{Display, Formatter, Result},
    path::PathBuf,
};

#[derive(Debug, Default, Clone)]
pub enum EntryKind {
    #[default]
    File,
    Dir,
    Symlink,
    Header,
}

impl Display for EntryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{self:?}") }
}

#[derive(Default, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: EntryKind,

    pub path_offset: usize,
}

pub struct PrintableEntry {
    pub perms: String,
    pub links: String,
    pub owner: String,
    pub group: String,
    pub size: String,
    pub modified: String,
    pub name: String,
}
