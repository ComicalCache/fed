use std::any::Any;
use std::fmt;

use crate::protocols::io::IoFileEntry;
use crate::protocols::picker::PickerEntry;
use crate::types::Theme;

impl fmt::Display for IoFileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.relative_path.display(), if self.dir { "/" } else { "" })
    }
}

impl PickerEntry for IoFileEntry {
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
