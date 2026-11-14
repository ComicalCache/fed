use crate::protocols::mp::PromptHistory;

#[derive(Default)]
pub struct ActionPState {
    pub align_history: PromptHistory,
}
