use std::fmt::Display;
use std::ops::RangeBounds;

use super::history::Commit;
use super::history::History;
use super::history::Kind;
use crate::piece_table::Chunks;

#[derive(Clone, Copy)]
pub(super) enum Source {
    Original,
    Addition,
}

#[derive(Clone, Copy)]
pub(super) struct Piece {
    pub(super) source: Source,
    pub(super) source_offset: usize,
    pub(super) byte_len: usize,
}

impl Piece {
    const fn new(source: Source, source_offset: usize, byte_len: usize) -> Self {
        Self { source, source_offset, byte_len }
    }
}

/// Piece table with a `History` to enable undo/redo operations.
pub struct PieceTable {
    /// Read-only input text.
    pub(super) original_buffer: String,
    /// Text added while editing.
    pub(super) add_buffer: String,
    /// List of pieces that point to data contained in text.
    // TODO: use B-tree.
    pub(super) pieces: Vec<Piece>,

    pub(super) line_starts: Vec<usize>,

    pub(super) byte_len: usize,

    pub(super) history: History,
    pub(super) active_commit: Option<Commit>,
}

impl PieceTable {
    pub fn start_commit(&mut self) {
        if self.active_commit.is_none() {
            self.active_commit = Some(Commit::new());
        }
    }

    pub fn end_commit(&mut self) {
        if let Some(commit) = self.active_commit.take()
            && !commit.changes.is_empty()
        {
            self.history.save(commit);
        }
    }

    /// Compacts the current text into a single original buffer, reclaiming
    /// storage.
    ///
    /// THIS OPERATION RESETS THE UNDO/REDO HISTORY! ANY ACTIVE COMMIT IS
    /// DISCARDED.
    pub fn compact(&mut self) { *self = Self::from(self.slice(..)); }

    pub fn insert<S: AsRef<str>>(&mut self, offset: usize, text: S) {
        assert!(offset <= self.byte_len,);

        let text = text.as_ref();
        assert!(!text.is_empty());

        let mut piece_offset = 0;
        let mut piece_idx = 0;
        let is_end = offset == self.byte_len;

        if is_end {
            piece_idx = self.pieces.len();
        } else if offset != 0 {
            let mut piece_start = 0;
            for (piece_jdx, piece) in self.pieces.iter().enumerate() {
                if offset < piece_start + piece.byte_len {
                    piece_idx = piece_jdx;
                    piece_offset = offset - piece_start;

                    break;
                }

                piece_start += piece.byte_len;
            }
        }

        // Extend existing `Piece` if possible.
        if (piece_offset == 0 || is_end) && piece_idx > 0 {
            let prev_piece_idx = piece_idx - 1;
            let piece = self.pieces[prev_piece_idx];

            if matches!(piece.source, Source::Addition)
                && piece.source_offset + piece.byte_len == self.add_buffer.len()
            {
                self.pieces[prev_piece_idx].byte_len += text.len();

                if let Some(commit) = &mut self.active_commit {
                    commit.add_change(prev_piece_idx, piece, Kind::Deletion);
                    commit.add_change(prev_piece_idx, self.pieces[prev_piece_idx], Kind::Insertion);
                    commit.add_logical_change(Kind::Insertion, offset, text.len());
                }

                self.byte_len += text.len();
                self.update_line_starts_insert(offset, text);
                self.add_buffer.push_str(text);

                return;
            }
        }

        let piece = Piece::new(Source::Addition, self.add_buffer.len(), text.len());
        let mut changes = Vec::new();

        if piece_offset == 0 || is_end {
            self.pieces.insert(piece_idx, piece);

            changes.push((piece_idx, piece, Kind::Insertion));
        } else {
            // Split existing piece into two and insert new piece between.
            let trailing_piece = Piece::new(
                self.pieces[piece_idx].source,
                self.pieces[piece_idx].source_offset + piece_offset,
                self.pieces[piece_idx].byte_len - piece_offset,
            );

            let original_piece = self.pieces[piece_idx];
            self.pieces[piece_idx].byte_len = piece_offset;

            changes.push((piece_idx, original_piece, Kind::Deletion));
            changes.push((piece_idx, self.pieces[piece_idx], Kind::Insertion));

            self.pieces.insert(piece_idx + 1, piece);
            changes.push((piece_idx + 1, piece, Kind::Insertion));

            self.pieces.insert(piece_idx + 2, trailing_piece);
            changes.push((piece_idx + 2, trailing_piece, Kind::Insertion));
        }

        if let Some(commit) = &mut self.active_commit {
            for (piece_index, piece, kind) in changes {
                commit.add_change(piece_index, piece, kind);
            }
            commit.add_logical_change(Kind::Insertion, offset, text.len());
        }

        self.byte_len += text.len();
        self.update_line_starts_insert(offset, text);
        self.add_buffer.push_str(text);
    }

    /// Appends text at the end of the document.
    pub fn append<S: AsRef<str>>(&mut self, text: S) { self.insert(self.byte_len, text); }

    pub fn remove(&mut self, offset: usize, byte_len: usize) {
        enum Removal {
            End { piece_idx: usize, byte_len: usize },
            Full { start_idx: usize, end_idx: usize },
            Start { piece_idx: usize, byte_len: usize },
            Slice { piece_idx: usize, piece_offset: usize },
        }

        let removal_end = offset + byte_len;
        assert!(removal_end <= self.byte_len);
        assert!(byte_len != 0);

        let mut removals = Vec::new();
        let mut piece_end = 0;
        for (piece_idx, piece) in self.pieces.iter().enumerate() {
            if piece_end >= removal_end {
                break;
            }

            let piece_start = piece_end;
            piece_end += piece.byte_len;

            let remove_start = offset <= piece_start;
            let remove_end = removal_end >= piece_end && piece_end > offset;
            let remove_slice = piece_start < offset && removal_end < piece_end;

            match (remove_start, remove_end) {
                (false, true) => {
                    removals.push(Removal::End { piece_idx, byte_len: piece_end - offset })
                }
                (true, true) => {
                    if let Some(Removal::Full { end_idx: end_index, .. }) = removals.last_mut() {
                        *end_index = piece_idx + 1;
                    } else {
                        removals
                            .push(Removal::Full { start_idx: piece_idx, end_idx: piece_idx + 1 });
                    }
                }
                (true, false) => {
                    removals.push(Removal::Start { piece_idx, byte_len: removal_end - piece_start })
                }
                _ if remove_slice => {
                    removals.push(Removal::Slice { piece_idx, piece_offset: offset - piece_start })
                }
                _ => {}
            }
        }

        self.byte_len -= byte_len;

        let mut changes = Vec::new();
        for removal in removals.iter().rev() {
            match removal {
                Removal::End { piece_idx, byte_len } => {
                    let original_piece = self.pieces[*piece_idx];

                    self.pieces[*piece_idx].byte_len -= *byte_len;

                    changes.push((*piece_idx, original_piece, Kind::Deletion));
                    changes.push((*piece_idx, self.pieces[*piece_idx], Kind::Insertion));
                }
                Removal::Full { start_idx: start_index, end_idx: end_index } => {
                    for (offset, piece) in
                        self.pieces.drain(*start_index..*end_index).enumerate().rev()
                    {
                        changes.push((*start_index + offset, piece, Kind::Deletion));
                    }
                }
                Removal::Start { piece_idx, byte_len } => {
                    let original_piece = self.pieces[*piece_idx];

                    self.pieces[*piece_idx].byte_len -= *byte_len;
                    self.pieces[*piece_idx].source_offset += *byte_len;

                    changes.push((*piece_idx, original_piece, Kind::Deletion));
                    changes.push((*piece_idx, self.pieces[*piece_idx], Kind::Insertion));
                }
                Removal::Slice { piece_idx, piece_offset } => {
                    let original_piece = self.pieces[*piece_idx];
                    let leading_piece = Piece::new(
                        original_piece.source,
                        original_piece.source_offset,
                        *piece_offset,
                    );
                    let trailing_piece = Piece::new(
                        original_piece.source,
                        original_piece.source_offset + *piece_offset + byte_len,
                        original_piece.byte_len - *piece_offset - byte_len,
                    );

                    self.pieces[*piece_idx] = leading_piece;
                    changes.push((*piece_idx, original_piece, Kind::Deletion));
                    changes.push((*piece_idx, leading_piece, Kind::Insertion));

                    self.pieces.insert(*piece_idx + 1, trailing_piece);
                    changes.push((*piece_idx + 1, trailing_piece, Kind::Insertion));
                }
            }
        }

        if let Some(commit) = &mut self.active_commit {
            for (piece_idx, piece, kind) in changes {
                commit.add_change(piece_idx, piece, kind);
            }
            commit.add_logical_change(Kind::Deletion, offset, byte_len);
        }

        self.update_line_starts_remove(offset, byte_len);
    }

    #[must_use]
    pub fn line_start_offset(&self, line_idx: usize) -> usize {
        self.line_starts.get(line_idx).copied().unwrap_or(self.byte_len)
    }

    /// Includes the newline when present, so this equals
    /// `line_start_offset(line_index + 1)`.
    #[must_use]
    pub fn line_end_offset(&self, line_idx: usize) -> usize {
        if line_idx + 1 < self.line_starts.len() {
            self.line_starts[line_idx + 1]
        } else {
            self.byte_len
        }
    }

    #[must_use]
    pub fn line_idx_at_offset(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|&line_idx| line_idx <= offset).saturating_sub(1)
    }

    #[must_use]
    pub const fn line_count(&self) -> usize { self.line_starts.len() }

    #[must_use]
    pub const fn len(&self) -> usize { self.byte_len }

    #[must_use]
    pub const fn is_empty(&self) -> bool { self.byte_len == 0 }

    /// Reverts the piece table to the state *before* the last changes. Returns
    /// logical edits (document byte offset, kind, data) in reverse operation
    /// order with inverted kinds. Offsets apply sequentially during undo.
    pub fn undo(&mut self) -> Vec<(usize, Kind, String)> {
        let Some(commit_idx) = self.history.undo() else {
            return Vec::new();
        };

        let commit = self.history.commit(commit_idx);
        let mut edits = Vec::with_capacity(commit.logical_changes.len());
        for logical_change in commit.logical_changes.iter().rev() {
            let text = if matches!(logical_change.kind, Kind::Insertion) {
                Some(self.slice(logical_change.offset..logical_change.offset + logical_change.len))
            } else {
                None
            };

            for change in commit.changes[logical_change.changes.clone()].iter().rev() {
                match change.kind {
                    Kind::Deletion => {
                        self.pieces.insert(change.piece_idx, change.piece);
                        self.byte_len += change.piece.byte_len;
                    }
                    Kind::Insertion => {
                        self.byte_len -= self.pieces.remove(change.piece_idx).byte_len;
                    }
                }
            }

            let kind = match logical_change.kind {
                Kind::Deletion => Kind::Insertion,
                Kind::Insertion => Kind::Deletion,
            };
            let text = text.unwrap_or_else(|| {
                self.slice(logical_change.offset..logical_change.offset + logical_change.len)
            });
            edits.push((logical_change.offset, kind, text));
        }

        // Fast forward single changes.
        if let [(offset, kind, text)] = &edits[..] {
            match kind {
                Kind::Deletion => {
                    self.update_line_starts_remove(*offset, text.len());
                }
                Kind::Insertion => self.update_line_starts_insert(*offset, text),
            }
        } else {
            self.rebuild_line_starts();
        }

        edits
    }

    /// Restores the piece table to the "hot" state *after* the last undo.
    /// Returns logical edits (document byte offset, kind, data) in original
    /// operation order. Offsets apply sequentially during redo.
    ///
    /// Hot state means the state the head was last at (e.g. at a fork in the
    /// history it can quickly be redone to the last head position without
    /// having to select it).
    pub fn hot_redo(&mut self) -> Vec<(usize, Kind, String)> {
        let Some(commit_idx) = self.history.hot_redo() else {
            return Vec::new();
        };

        let commit = self.history.commit(commit_idx);
        let mut edits = Vec::with_capacity(commit.logical_changes.len());
        for logical_change in &commit.logical_changes {
            let text = if matches!(logical_change.kind, Kind::Deletion) {
                Some(self.slice(logical_change.offset..logical_change.offset + logical_change.len))
            } else {
                None
            };

            for change in &commit.changes[logical_change.changes.clone()] {
                match change.kind {
                    Kind::Deletion => {
                        self.byte_len -= self.pieces.remove(change.piece_idx).byte_len;
                    }
                    Kind::Insertion => {
                        self.pieces.insert(change.piece_idx, change.piece);
                        self.byte_len += change.piece.byte_len;
                    }
                }
            }

            let text = text.unwrap_or_else(|| {
                self.slice(logical_change.offset..logical_change.offset + logical_change.len)
            });
            edits.push((logical_change.offset, logical_change.kind, text));
        }

        // Fast forward single changes.
        if let [(offset, kind, text)] = &edits[..] {
            match kind {
                Kind::Deletion => {
                    self.update_line_starts_remove(*offset, text.len());
                }
                Kind::Insertion => self.update_line_starts_insert(*offset, text),
            }
        } else {
            self.rebuild_line_starts();
        }

        edits
    }

    /// Returns an owned slice of the text stored in the `PieceTable`.
    #[must_use]
    pub fn slice(&self, range: impl RangeBounds<usize>) -> String {
        let range = self.normalize_range(range);
        let mut text = String::with_capacity(range.end - range.start);

        let chunks = self.chunks(range);
        for chunk in chunks {
            text.push_str(chunk);
        }

        text
    }

    fn update_line_starts_insert(&mut self, offset: usize, text: &str) {
        let mut added_line_starts = Vec::new();
        for (byte_offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                added_line_starts.push(offset + byte_offset + 1);
            }
        }

        let insert_idx = self.line_starts.partition_point(|&line_idx| line_idx <= offset);

        for line_start in &mut self.line_starts[insert_idx..] {
            *line_start += text.len();
        }

        self.line_starts.splice(insert_idx..insert_idx, added_line_starts);
    }

    fn update_line_starts_remove(&mut self, offset: usize, len: usize) {
        let removal_end = offset + len;

        let start_idx = self.line_starts.partition_point(|&line_idx| line_idx <= offset);
        let end_idx = self.line_starts.partition_point(|&line_idx| line_idx <= removal_end);

        self.line_starts.drain(start_idx..end_idx);

        for line_start in &mut self.line_starts[start_idx..] {
            *line_start -= len;
        }
    }

    fn rebuild_line_starts(&mut self) {
        let mut line_starts = vec![0];

        let mut offset = 0;
        for piece in &self.pieces {
            let buffer = match piece.source {
                Source::Original => &self.original_buffer,
                Source::Addition => &self.add_buffer,
            };

            let text = &buffer[piece.source_offset..piece.source_offset + piece.byte_len];
            for (byte_offset, byte) in text.bytes().enumerate() {
                if byte == b'\n' {
                    line_starts.push(offset + byte_offset + 1);
                }
            }

            offset += piece.byte_len;
        }

        self.line_starts = line_starts;
    }
}

impl From<&str> for PieceTable {
    fn from(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }

        let original_buffer = String::from(text);
        let byte_len = original_buffer.len();
        let pieces = if original_buffer.is_empty() {
            vec![]
        } else {
            vec![Piece::new(Source::Original, 0, byte_len)]
        };

        Self {
            original_buffer,
            add_buffer: String::new(),
            pieces,
            line_starts,
            byte_len,
            history: History::new(Commit::new()),
            active_commit: None,
        }
    }
}

impl From<String> for PieceTable {
    fn from(text: String) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }

        let byte_len = text.len();
        let pieces =
            if text.is_empty() { vec![] } else { vec![Piece::new(Source::Original, 0, byte_len)] };

        Self {
            original_buffer: text,
            add_buffer: String::new(),
            pieces,
            line_starts,
            byte_len,
            history: History::new(Commit::new()),
            active_commit: None,
        }
    }
}

impl Default for PieceTable {
    fn default() -> Self { Self::from("") }
}

impl Display for PieceTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.chunks(..) {
            f.write_str(chunk)?;
        }

        Ok(())
    }
}
