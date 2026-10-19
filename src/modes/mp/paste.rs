use crate::{
    fed::FCmd,
    protocols::{action::ActionCmd, mp::MpKind},
    state::State,
    types::{PasteInputHandler, PasteInputPriority},
};

pub struct MpPasteInput {}

impl PasteInputHandler for MpPasteInput {
    fn priority(&self) -> PasteInputPriority { PasteInputPriority::MpMode }

    fn paste(&mut self, s: &State, text: &str) -> Option<Vec<FCmd>> {
        if s.p.mp.window.is_none() || s.workspace.active_window != s.p.mp.window {
            return None;
        }
        if s.p.mp.kind != MpKind::Prompt {
            return Some(Vec::new());
        }

        let text: String = text.chars().filter(|&ch| !ch.is_control() || ch == '\t').collect();
        if text.is_empty() {
            return Some(Vec::new());
        }

        Some(vec![FCmd::Action(ActionCmd::Insert { view: s.p.mp.view, text })])
    }
}
