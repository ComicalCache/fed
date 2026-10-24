use crate::fed::FCmd;
use crate::protocols::action::ActionCmd;
use crate::protocols::picker::PickerCmd;
use crate::state::State;
use crate::types::PasteInputHandler;
use crate::types::PasteInputPriority;
use crate::util::text::sanitize;

pub struct PickerPasteInput {}

impl PasteInputHandler for PickerPasteInput {
    fn priority(&self) -> PasteInputPriority { PasteInputPriority::PickerMode }

    fn paste(&mut self, s: &State, text: &str) -> Option<Vec<FCmd>> {
        if s.p.picker.window.is_none() || s.workspace.active_window != s.p.picker.window {
            return None;
        }

        let text = sanitize(text, |ch| ch == '\t');
        if text.is_empty() {
            return Some(Vec::new());
        }

        Some(vec![
            FCmd::Action(ActionCmd::Insert { view: s.p.picker.input_view, text }),
            FCmd::Picker(PickerCmd::FilterList),
        ])
    }
}
