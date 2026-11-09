use crate::protocols::picker::PCallback;
use crate::protocols::picker::PickerEntry;
use crate::types::Motion;

#[allow(unused)]
pub enum PickerCmd {
    Open { width: usize, height: usize, entries: Vec<Box<dyn PickerEntry>>, callback: PCallback },
    Close,

    FilterList,
    FilterCompletion { filtered: Vec<usize>, revision: u64 },
    ChangeSelection(Motion),
    ScrollList(Motion),
    Select,

    Resize { width: usize, height: usize },
}
