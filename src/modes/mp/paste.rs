use crate::fed::FCmd;
use crate::protocols::action::ActionCmd;
use crate::protocols::mp::MpKind;
use crate::state::State;
use crate::types::PasteInputHandler;
use crate::types::PasteInputPriority;
use crate::util::text::sanitize;

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

        let text = sanitize(text, |ch| ch == '\t');
        if text.is_empty() {
            return Some(Vec::new());
        }

        Some(vec![FCmd::Action(ActionCmd::Insert { view: s.p.mp.view, text })])
    }
}
