use crate::protocols::mp::PromptHistory;

#[derive(Default)]
pub struct ViewPState {
    pub jump_history: PromptHistory,
}
