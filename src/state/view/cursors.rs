use crate::types::Cursor;

#[derive(Clone)]
pub struct Cursors {
    pub list: Vec<Cursor>,
}

impl Cursors {
    pub fn normalize(&mut self) {
        self.list.sort_unstable_by_key(|c| c.offset);
        self.list.dedup_by_key(|c| c.offset);
    }
}

impl Default for Cursors {
    fn default() -> Self { Self { list: vec![Cursor::default()] } }
}
