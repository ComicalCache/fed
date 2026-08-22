use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use crate::piece_table::PieceTable;

pub trait Slice<T> {
    /// Returns a slice of the text stored in the `PieceTable`.
    fn slice(&self, idx: T) -> String;
}

impl Slice<Range<usize>> for PieceTable {
    /// Returns the text stored in the `PieceTable` from pos `start..end`.
    ///
    /// # Panic
    /// Panics if
    /// - Range is out of bounds
    /// - `end <= start`
    fn slice(&self, idx: Range<usize>) -> String { self.__slice(idx.start, idx.end) }
}

impl Slice<RangeFrom<usize>> for PieceTable {
    /// Returns the text stored in the `PieceTable` from pos `start..`.
    ///
    /// # Panic
    /// Panics if range is out of bounds.
    fn slice(&self, idx: RangeFrom<usize>) -> String { self.__slice(idx.start, self.total_length) }
}

impl Slice<RangeFull> for PieceTable {
    /// Returns the entire text stored in the `PieceTable` (prefer
    /// `PieceTable::to_string`).
    fn slice(&self, _: RangeFull) -> String { self.__slice(0, self.total_length) }
}

impl Slice<RangeInclusive<usize>> for PieceTable {
    /// Returns the text stored in the `PieceTable` from pos `start..=end`.
    ///
    /// # Panic
    /// Panics if
    /// - Range is out of bounds
    /// - `end + 1 <= start`
    /// - `end == usize::MAX`
    fn slice(&self, idx: RangeInclusive<usize>) -> String {
        assert!(*idx.end() != usize::MAX, "Upper bound must not be usize::MAX");

        self.slice(*idx.start()..*idx.end() + 1)
    }
}

impl Slice<RangeTo<usize>> for PieceTable {
    /// Returns the text stored in the `PieceTable` from pos `..end`.
    ///
    ///
    /// # Panic
    /// Panics if range is out of bounds.
    fn slice(&self, idx: RangeTo<usize>) -> String { self.slice(0..idx.end) }
}

impl Slice<RangeToInclusive<usize>> for PieceTable {
    /// Returns the text stored in the `PieceTable` from pos `..=end`.
    ///
    /// # Panic
    /// Panics if
    /// - Range is out of bounds
    /// - `end == usize::MAX`
    fn slice(&self, idx: RangeToInclusive<usize>) -> String {
        assert!(idx.end != usize::MAX, "Upper incluse index must not be usize::MAX");

        self.slice(0..idx.end + 1)
    }
}
