use std::iter::FusedIterator;
use std::ops::Bound;
use std::ops::Range;
use std::ops::RangeBounds;
use std::slice::Iter;

use crate::piece_table::PieceTable;
use crate::piece_table::piece_table::Piece;
use crate::piece_table::piece_table::Source;

pub trait Chunks<T> {
    /// Returns borrowed chunks of the text stored in the `PieceTable`.
    /// Chunk boundaries follow pieces, not lines or grapheme clusters.
    fn chunks(&self, range: T) -> ChunkIter<'_>;
}

impl PieceTable {
    pub(super) fn normalize_range(&self, range: impl RangeBounds<usize>) -> Range<usize> {
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => self.byte_len,
        };

        assert!(start <= end);
        assert!(end <= self.byte_len);

        start..end
    }
}

impl<T: RangeBounds<usize>> Chunks<T> for PieceTable {
    fn chunks(&self, range: T) -> ChunkIter<'_> {
        let range = self.normalize_range(range);
        ChunkIter {
            table: self,
            pieces: self.pieces.iter(),
            remaining_start: 0,
            remaining_end: self.byte_len,
            range_start: range.start,
            range_end: range.end,
        }
    }
}

#[derive(Clone)]
pub struct ChunkIter<'a> {
    table: &'a PieceTable,
    pieces: Iter<'a, Piece>,
    remaining_start: usize,
    remaining_end: usize,
    range_start: usize,
    range_end: usize,
}

impl<'a> ChunkIter<'a> {
    fn piece_text(&self, piece: Piece, local_start: usize, local_end: usize) -> &'a str {
        let buffer = match piece.source {
            Source::Original => &self.table.original_buffer,
            Source::Addition => &self.table.add_buffer,
        };
        &buffer[piece.source_offset + local_start..piece.source_offset + local_end]
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range_start == self.range_end {
            return None;
        }

        while self.remaining_start < self.range_end && self.remaining_end > self.range_start {
            let piece = *self.pieces.next()?;
            let piece_start = self.remaining_start;
            self.remaining_start += piece.byte_len;

            if self.remaining_start > self.range_start && piece_start < self.range_end {
                return Some(self.piece_text(
                    piece,
                    self.range_start.saturating_sub(piece_start),
                    piece.byte_len.min(self.range_end - piece_start),
                ));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(self.pieces.len())) }
}

impl DoubleEndedIterator for ChunkIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.range_start == self.range_end {
            return None;
        }

        while self.remaining_start < self.range_end && self.remaining_end > self.range_start {
            let piece = *self.pieces.next_back()?;
            self.remaining_end -= piece.byte_len;

            if self.remaining_end < self.range_end
                && self.remaining_end + piece.byte_len > self.range_start
            {
                return Some(self.piece_text(
                    piece,
                    self.range_start.saturating_sub(self.remaining_end),
                    piece.byte_len.min(self.range_end - self.remaining_end),
                ));
            }
        }
        None
    }
}

impl FusedIterator for ChunkIter<'_> {}
