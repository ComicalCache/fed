use std::sync::{Arc, atomic::AtomicUsize};

#[derive(Default)]
pub struct IoState {
    pub writers: Arc<AtomicUsize>,
}
