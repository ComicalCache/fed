use crate::{
    protocols::picker::{PCallback, PickerEntry},
    types::Motion,
};

#[allow(unused)]
pub enum PickerCmd {
    Open { width: usize, height: usize, entries: Vec<Box<dyn PickerEntry>>, callback: PCallback },
    Close,

    FilterList,
    ChangeSelection(Motion),
    ScrollList(Motion),
    Select,

    Resize { width: usize, height: usize },
}
