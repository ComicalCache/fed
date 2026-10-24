use std::iter::FusedIterator;
use std::ops::Bound;
use std::ops::Range;
use std::ops::RangeBounds;
use std::slice::Iter;

use crate::piece_table::Piece;
use crate::piece_table::PieceTable;
use crate::piece_table::Source;

pub trait Chunks<T> {
    /// Returns borrowed chunks of the text stored in the `PieceTable`.
    /// Chunk boundaries follow pieces, not lines or grapheme clusters.
    fn chunks(&self, idx: T) -> ChunkIter<'_>;
}

impl PieceTable {
    pub(super) fn normalize_range(&self, idx: impl RangeBounds<usize>) -> Range<usize> {
        let start = match idx.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start.checked_add(1).expect("Lower bound must not overflow"),
            Bound::Unbounded => 0,
        };
        let end = match idx.end_bound() {
            Bound::Included(&end) => end.checked_add(1).expect("Upper bound must not overflow"),
            Bound::Excluded(&end) => end,
            Bound::Unbounded => self.total_length,
        };
        assert!(start <= end, "Lower range bound must not exceed upper bound");
        assert!(end <= self.total_length, "Range must be within the text");
        start..end
    }
}

impl<T: RangeBounds<usize>> Chunks<T> for PieceTable {
    fn chunks(&self, idx: T) -> ChunkIter<'_> {
        let range = self.normalize_range(idx);
        ChunkIter {
            table: self,
            pieces: self.pieces.iter(),
            front: 0,
            back: self.total_length,
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Clone)]
pub struct ChunkIter<'a> {
    table: &'a PieceTable,
    pieces: Iter<'a, Piece>,
    front: usize,
    back: usize,
    start: usize,
    end: usize,
}

impl<'a> ChunkIter<'a> {
    fn text(&self, piece: Piece, start: usize, end: usize) -> &'a str {
        let source = match piece.source {
            Source::Original => &self.table.original,
            Source::Addition => &self.table.addition,
        };
        &source[piece.offset + start..piece.offset + end]
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        while self.front < self.end && self.back > self.start {
            let piece = *self.pieces.next()?;
            let offset = self.front;
            self.front += piece.length;

            if self.front > self.start && offset < self.end {
                return Some(self.text(
                    piece,
                    self.start.saturating_sub(offset),
                    piece.length.min(self.end - offset),
                ));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(self.pieces.len())) }
}

impl DoubleEndedIterator for ChunkIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        while self.front < self.end && self.back > self.start {
            let piece = *self.pieces.next_back()?;
            self.back -= piece.length;

            if self.back < self.end && self.back + piece.length > self.start {
                return Some(self.text(
                    piece,
                    self.start.saturating_sub(self.back),
                    piece.length.min(self.end - self.back),
                ));
            }
        }
        None
    }
}

impl FusedIterator for ChunkIter<'_> {}
