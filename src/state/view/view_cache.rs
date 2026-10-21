#[derive(Default, Clone)]
pub struct ViewCache {
    pub offset: usize,
    pub lines: Vec<String>,
    pub revision: Option<(u64, usize, usize)>,
}
