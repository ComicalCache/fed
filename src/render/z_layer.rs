#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ZLayer {
    // Needs the highest layer to always be on top.
    MiniBuffer,
}
