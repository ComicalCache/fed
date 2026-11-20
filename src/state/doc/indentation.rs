#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Indentation {
    #[default]
    Tabs,
    Spaces,
}
