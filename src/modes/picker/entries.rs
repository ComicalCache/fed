use std::{any::Any, fmt, path::PathBuf};

use crate::{protocols::picker::PickerEntry, types::Theme};

#[derive(Clone)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub dir: bool,
}

impl fmt::Display for ScanEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.path.display(), if self.dir { "/" } else { "" })
    }
}

impl PickerEntry for ScanEntry {
    fn as_any(&self) -> &dyn Any { self }

    fn into_any(self: Box<Self>) -> Box<dyn Any + Send> { self }
}

pub struct ThemeEntry {
    pub name: &'static str,
    pub theme: Theme,
}

impl fmt::Display for ThemeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.name) }
}

impl PickerEntry for ThemeEntry {
    fn as_any(&self) -> &dyn Any { self }

    fn into_any(self: Box<Self>) -> Box<dyn Any + Send> { self }
}
