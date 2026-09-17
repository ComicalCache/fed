#[derive(Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    None,
    Message,
    Prompt,
}
