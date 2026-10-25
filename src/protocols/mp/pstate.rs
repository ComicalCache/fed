use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::types::render::WindowId;
use crate::util::newtype;

pub type PCallback = Box<dyn FnOnce(&mut State, Option<String>) + Send>;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum MpKind {
    #[default]
    None,
    Message,
    Prompt,
}

newtype!(MpId, usize);

pub struct MpPState {
    pub next_id: usize,
    pub mp: MpId,

    pub kind: MpKind,

    pub doc: DocId,
    pub view: ViewId,

    pub window: Option<WindowId>,
    pub prev_window: Option<WindowId>,

    pub width: usize,
    pub height: usize,

    pub pcallback: Option<PCallback>,
}

impl MpPState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            next_id: 1,
            mp: MpId(0),
            kind: MpKind::default(),
            doc: DocId::default(),
            view: ViewId::default(),
            window: None,
            prev_window: None,
            width,
            height,
            pcallback: None,
        }
    }
}
