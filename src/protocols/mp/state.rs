use crate::{protocols::state::PState, state::State};

pub type PCallback = Box<dyn FnOnce(&mut State, &mut PState, String) + Send>;

pub struct MpState {
    pub next_id: usize,

    pub width: usize,
    pub height: usize,

    pub pcallback: Option<PCallback>,
}

impl MpState {
    pub fn new(width: usize, height: usize) -> Self {
        Self { next_id: 1, width, height, pcallback: None }
    }
}
