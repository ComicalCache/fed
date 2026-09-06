pub mod slice;

use std::fmt::Display;

use crate::history::{Commit, History, Kind};

#[derive(Clone, Copy)]
enum Source {
    Original,
    Addition,
}

#[derive(Clone, Copy)]
pub struct Piece {
    source: Source,
    /// Offset in the data source.
    offset: usize,
    /// Length of pointed at content.
    length: usize,
}

impl Piece {
    const fn new(source: Source, offset: usize, length: usize) -> Self {
        Self { source, offset, length }
    }
}

/// Piece table with a `History` to enable undo/redo operations.
pub struct PieceTable {
    /// Read only input data.
    original: String,
    /// Data added while editing.
    addition: String,
    /// List of pieces that point to data contained in text.
    // TODO: use B-tree.
    pieces: Vec<Piece>,

    /// Start bytes of lines.
    lines: Vec<usize>,

    /// Length of the text contained in the piece table.
    total_length: usize,

    /// Edit history.
    history: History,

    /// The currently active transaction.
    active_commit: Option<Commit>,
}

impl PieceTable {
    /// Starts a new commit transaction.
    pub fn start_commit(&mut self) {
        if self.active_commit.is_none() {
            self.active_commit = Some(Commit::new());
        }
    }

    /// Ends the current commit transaction.
    pub fn end_commit(&mut self) {
        if let Some(commit) = self.active_commit.take()
            && !commit.changes.is_empty()
        {
            self.history.save(commit);
        }
    }

    /// Inserts `string` at `pos`.
    pub fn insert<S: AsRef<str>>(&mut self, mut pos: usize, str: S) {
        assert!(
            pos <= self.total_length,
            "Insert position must be within or at the end of the text"
        );

        let str = str.as_ref();
        assert!(!str.is_empty(), "Inserted string must not be empty");

        let update_pos = pos;

        let mut idx = 0;
        let eof = pos == self.total_length;

        if eof {
            idx = self.pieces.len();
        } else if pos != 0 {
            let mut len = 0;
            for (jdx, piece) in self.pieces.iter().enumerate() {
                if pos < len + piece.length {
                    idx = jdx;
                    pos -= len;

                    break;
                }

                len += piece.length;
            }
        }

        // Extend existing `Piece` if possible.
        if (pos == 0 || eof) && idx > 0 {
            let idx = idx - 1;
            let piece = self.pieces[idx];

            if matches!(piece.source, Source::Addition)
                && piece.offset + piece.length == self.addition.len()
            {
                self.pieces[idx].length += str.len();

                if let Some(commit) = &mut self.active_commit {
                    commit.add_change(idx, piece, Kind::Deletion);
                    commit.add_change(idx, self.pieces[idx], Kind::Insertion);
                }

                self.total_length += str.len();
                self.update_lines_insert(update_pos, str);
                self.addition.push_str(str);

                return;
            }
        }

        let piece = Piece::new(Source::Addition, self.addition.len(), str.len());
        let mut changes = Vec::new();

        if pos == 0 || eof {
            self.pieces.insert(idx, piece);

            changes.push((idx, piece, Kind::Insertion));
        } else {
            // Split existing piece into two and insert new piece between.
            let trailing = Piece::new(
                self.pieces[idx].source,
                self.pieces[idx].offset + pos,
                self.pieces[idx].length - pos,
            );

            let old = self.pieces[idx];
            self.pieces[idx].length = pos;

            changes.push((idx, old, Kind::Deletion));
            changes.push((idx, self.pieces[idx], Kind::Insertion));

            self.pieces.insert(idx + 1, piece);
            changes.push((idx + 1, piece, Kind::Insertion));

            self.pieces.insert(idx + 2, trailing);
            changes.push((idx + 2, trailing, Kind::Insertion));
        }

        if let Some(commit) = &mut self.active_commit {
            for (idx, piece, kind) in changes {
                commit.add_change(idx, piece, kind);
            }
        }

        self.total_length += str.len();
        self.update_lines_insert(update_pos, str);
        self.addition.push_str(str);
    }

    /// Appends a string at the end.
    pub fn append<S: AsRef<str>>(&mut self, str: S) { self.insert(self.total_length, str); }

    /// Removes a string from `pos` of length `n`.
    pub fn remove(&mut self, pos: usize, n: usize) {
        enum Remove {
            End { index: usize, length: usize },
            Full { index: usize },
            Start { index: usize, length: usize },
            Slice { index: usize, offset: usize },
        }

        let end = pos + n;
        assert!(end <= self.total_length, "Removed string must be within the text");
        assert!(n != 0, "Must remove at least 1 character");

        let mut remove = Vec::new();
        let mut len = 0;
        for (index, piece) in self.pieces.iter().enumerate() {
            if len >= end {
                break;
            }

            let prev_len = len;
            len += piece.length;

            let remove_start = pos <= prev_len;
            let remove_end = end >= len && len > pos;
            let remove_slice = prev_len < pos && end < len;

            match (remove_start, remove_end) {
                (false, true) => remove.push(Remove::End { index, length: len - pos }),
                (true, true) => remove.push(Remove::Full { index }),
                (true, false) => remove.push(Remove::Start { index, length: end - prev_len }),
                _ if remove_slice => remove.push(Remove::Slice { index, offset: pos - prev_len }),
                _ => {}
            }
        }

        self.total_length -= n;

        let mut changes = Vec::new();
        for piece in remove.iter().rev() {
            match piece {
                Remove::End { index, length } => {
                    let old = self.pieces[*index];

                    self.pieces[*index].length -= *length;

                    changes.push((*index, old, Kind::Deletion));
                    changes.push((*index, self.pieces[*index], Kind::Insertion));
                }
                Remove::Full { index } => {
                    changes.push((*index, self.pieces.remove(*index), Kind::Deletion));
                }
                Remove::Start { index, length } => {
                    let old = self.pieces[*index];

                    self.pieces[*index].length -= *length;
                    self.pieces[*index].offset += *length;

                    changes.push((*index, old, Kind::Deletion));
                    changes.push((*index, self.pieces[*index], Kind::Insertion));
                }
                Remove::Slice { index, offset } => {
                    let old = self.pieces[*index];
                    let leading = Piece::new(old.source, old.offset, *offset);
                    let trailing_len = old.length - *offset - n;
                    let trailing = Piece::new(old.source, old.offset + *offset + n, trailing_len);

                    self.pieces[*index] = leading;
                    changes.push((*index, old, Kind::Deletion));
                    changes.push((*index, leading, Kind::Insertion));

                    self.pieces.insert(*index + 1, trailing);
                    changes.push((*index + 1, trailing, Kind::Insertion));
                }
            }
        }

        if let Some(commit) = &mut self.active_commit {
            for (idx, piece, kind) in changes {
                commit.add_change(idx, piece, kind);
            }
        }

        self.update_lines_remove(pos, n);
    }

    /// Returns the byte index where the nth line begins.
    pub fn get_line_start_byte(&self, n: usize) -> usize {
        self.lines.get(n).copied().unwrap_or(self.total_length)
    }

    /// Returns the byte index of where the nth line ends and the next line
    /// begins (`get_line_end_byte(n) == get_line_start_byte(n + 1)`).
    pub fn get_line_end_byte(&self, n: usize) -> usize {
        if n + 1 < self.lines.len() { self.lines[n + 1] } else { self.total_length }
    }

    /// Returns the amonut of lines of the text stored in the piece table.
    #[must_use]
    pub const fn lines(&self) -> usize { self.lines.len() }

    /// Returns the length of the text stored in the piece table.
    #[must_use]
    pub const fn len(&self) -> usize { self.total_length }

    /// Returns if the text stored in the piece table is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool { self.total_length == 0 }

    /// Reverts the piece table to the state *before* the last changes.
    pub fn undo(&mut self) {
        let Some(commit) = self.history.undo() else {
            return;
        };

        for change in commit.changes.iter().rev() {
            match change.kind {
                Kind::Deletion => {
                    self.pieces.insert(change.pos, change.piece);
                    self.total_length += change.piece.length;
                }
                Kind::Insertion => {
                    self.total_length -= self.pieces.remove(change.pos).length;
                }
            }
        }

        self.rebuild_lines();
    }

    /// Restores the piece table to the "hot" state *after* the last undo.
    ///
    /// Hot state means the state the head was last at (e.g. at a fork in the
    /// history it can quickly be redone to the last head position without
    /// having to select it).
    pub fn hot_redo(&mut self) {
        let Some(commit) = self.history.hot_redo() else {
            return;
        };

        for change in &commit.changes {
            match change.kind {
                Kind::Deletion => {
                    self.total_length -= self.pieces.remove(change.pos).length;
                }
                Kind::Insertion => {
                    self.pieces.insert(change.pos, change.piece);
                    self.total_length += change.piece.length;
                }
            }
        }

        self.rebuild_lines();
    }

    /// Returns text stored in the piece table (`upper` is exclusive).
    pub(crate) fn __slice(&self, lower: usize, upper: usize) -> String {
        assert!(lower <= upper, "Lower slice bound must be smaller than upper");

        let mut out = String::with_capacity(upper - lower);
        if lower == upper {
            return out;
        }

        let mut len = 0;
        for piece in &self.pieces {
            if len >= upper {
                break;
            }

            let prev_len = len;
            len += piece.length;

            let source = match piece.source {
                Source::Original => &self.original,
                Source::Addition => &self.addition,
            };

            let capture_start = lower <= prev_len;
            let capture_end = upper >= len && len > lower;
            let caputre_slice = prev_len < lower && upper < len;

            match (capture_start, capture_end) {
                (false, true) => {
                    let start = piece.offset + (lower - prev_len);
                    let end = piece.offset + piece.length;

                    out.push_str(&source[start..end]);
                }
                (true, true) => out.push_str(&source[piece.offset..piece.offset + piece.length]),
                (true, false) => {
                    let start = piece.offset;
                    let end = piece.offset + (upper - prev_len);

                    out.push_str(&source[start..end]);
                }
                _ if caputre_slice => {
                    let start = piece.offset + (lower - prev_len);
                    let end = piece.offset + (upper - prev_len);

                    out.push_str(&source[start..end]);
                }
                _ => {}
            }
        }

        out
    }

    fn update_lines_insert(&mut self, pos: usize, text: &str) {
        let mut lines = Vec::new();
        for (idx, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                lines.push(pos + idx + 1);
            }
        }

        let lines_idx = self.lines.partition_point(|&x| x <= pos);

        for offset in &mut self.lines[lines_idx..] {
            *offset += text.len();
        }

        self.lines.splice(lines_idx..lines_idx, lines);
    }

    fn update_lines_remove(&mut self, pos: usize, n: usize) {
        let end = pos + n;

        let start = self.lines.partition_point(|&x| x <= pos);
        let end = self.lines.partition_point(|&x| x <= end);

        self.lines.drain(start..end);

        for offset in &mut self.lines[start..] {
            *offset -= n;
        }
    }

    fn rebuild_lines(&mut self) {
        let mut lines = vec![0];

        let mut pos = 0;
        for piece in &self.pieces {
            let source = match piece.source {
                Source::Original => &self.original,
                Source::Addition => &self.addition,
            };

            let text = &source[piece.offset..piece.offset + piece.length];
            for (idx, byte) in text.bytes().enumerate() {
                if byte == b'\n' {
                    lines.push(pos + idx + 1);
                }
            }

            pos += piece.length;
        }

        self.lines = lines;
    }
}

impl From<&str> for PieceTable {
    fn from(str: &str) -> Self {
        let mut lines = vec![0];
        for (idx, byte) in str.bytes().enumerate() {
            if byte == b'\n' {
                lines.push(idx + 1);
            }
        }

        let string = String::from(str);
        let len = string.len();
        let pieces =
            if string.is_empty() { vec![] } else { vec![Piece::new(Source::Original, 0, len)] };

        Self {
            original: string,
            addition: String::new(),
            pieces,
            lines,
            total_length: len,
            history: History::new(Commit::new()),
            active_commit: None,
        }
    }
}

impl From<String> for PieceTable {
    fn from(str: String) -> Self {
        let mut lines = vec![0];
        for (idx, byte) in str.bytes().enumerate() {
            if byte == b'\n' {
                lines.push(idx + 1);
            }
        }

        let len = str.len();
        let pieces =
            if str.is_empty() { vec![] } else { vec![Piece::new(Source::Original, 0, len)] };

        Self {
            original: str,
            addition: String::new(),
            pieces,
            lines,
            total_length: len,
            history: History::new(Commit::new()),
            active_commit: None,
        }
    }
}

impl Default for PieceTable {
    fn default() -> Self { PieceTable::from("") }
}

impl Display for PieceTable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.__slice(0, self.total_length))
    }
}
