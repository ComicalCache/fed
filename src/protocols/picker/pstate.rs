use std::any::Any;
use std::fmt::Display;

use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::types::render::WindowId;

pub trait PickerEntry: Display + Send + 'static {
    fn as_any(&self) -> &dyn Any;

    fn into_any(self: Box<Self>) -> Box<dyn Any + Send>;
}

pub type PCallback = Box<dyn FnOnce(&mut State, Option<Box<dyn PickerEntry>>) + Send>;

#[derive(Default)]
pub struct PickerPState {
    pub input_doc: DocId,
    pub input_view: ViewId,
    pub list_doc: DocId,
    pub list_view: ViewId,

    pub pref_width: usize,
    pub pref_height: usize,

    pub window: Option<WindowId>,
    pub prev_window: Option<WindowId>,

    pub entries: Vec<Box<dyn PickerEntry>>,
    pub display_text: Vec<String>,
    pub search_text: Vec<String>,
    pub filtered: Vec<usize>,
    pub offset: usize,
    pub pcallback: Option<PCallback>,
}
