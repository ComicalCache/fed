use std::sync::{Arc, atomic::AtomicUsize};

#[derive(Default)]
pub struct IoPState {
    pub writers: Arc<AtomicUsize>,
}
