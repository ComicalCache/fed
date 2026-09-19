macro_rules! debug_panic {
    ($($arg:tt)*) => { if cfg!(debug_assertions) { panic!($($arg)*) } };
}
pub(crate) use debug_panic;
