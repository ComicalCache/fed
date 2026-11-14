#[derive(Default, Clone)]
pub struct ViewCache {
    pub offset: usize,
    pub lines: Vec<String>,
    pub rev: Option<(u64, usize, usize)>,
}
