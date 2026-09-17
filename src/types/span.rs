pub struct Span<T> {
    pub start: usize,
    pub end: usize,
    pub data: T,
}
