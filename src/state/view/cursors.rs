use crate::types::Cursor;

#[derive(Clone)]
pub struct Cursors {
    pub list: Vec<Cursor>,
}

impl Default for Cursors {
    fn default() -> Self { Self { list: vec![Cursor::default()] } }
}
