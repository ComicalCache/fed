use std::ops::Range;

use crate::piece_table::piece_table::Piece;

#[derive(Clone, Copy)]
pub enum Kind {
    Deletion,
    Insertion,
}

pub(super) struct Change {
    pub(super) piece_idx: usize,
    pub(super) piece: Piece,
    pub(super) kind: Kind,
}

impl Change {
    pub const fn new(piece_idx: usize, piece: Piece, kind: Kind) -> Self {
        Self { piece_idx, piece, kind }
    }
}

pub(super) struct LogicalChange {
    pub(super) kind: Kind,
    pub(super) offset: usize,
    pub(super) len: usize,

    pub(super) changes: Range<usize>,
}

pub(super) struct Commit {
    pub(super) changes: Vec<Change>,
    pub(super) logical_changes: Vec<LogicalChange>,
}

impl Commit {
    pub const fn new() -> Self { Self { changes: Vec::new(), logical_changes: Vec::new() } }

    pub fn add_change(&mut self, pos: usize, piece: Piece, kind: Kind) {
        self.changes.push(Change::new(pos, piece, kind));
    }

    pub fn add_logical_change(&mut self, kind: Kind, offset: usize, len: usize) {
        let start = self.logical_changes.last().map_or(0, |c| c.changes.end);
        self.logical_changes.push(LogicalChange {
            kind,
            offset,
            len,
            changes: start..self.changes.len(),
        });
    }
}

struct Entry {
    /// Index of the previous/following (if any) entries in the history.
    previous: Option<usize>,
    next: Vec<usize>,

    /// Path of last head location. Enables hot redo that greedily redos to the
    /// last head position.
    hot_path: Option<usize>,

    commit: Commit,
}

impl Entry {
    const fn new(commit: Commit) -> Self {
        Self { previous: None, next: Vec::new(), hot_path: None, commit }
    }
}

/// Manipulation history of a `Piecetable`.
///
/// Creates a tree of changes that can be traversed forward and backward.
pub(super) struct History {
    entries: Vec<Entry>,
    head: usize,
}

impl History {
    pub(super) fn new(commit: Commit) -> Self {
        Self { entries: vec![Entry::new(commit)], head: 0 }
    }

    /// Updates the history and adds the latest commit.
    pub(super) fn save(&mut self, commit: Commit) {
        let prev_head = self.head;
        self.head = self.entries.len();

        self.entries[prev_head].next.push(self.head);

        let mut entry = Entry::new(commit);
        entry.previous = Some(prev_head);
        self.entries.push(entry);
    }

    pub(super) fn commit(&self, index: usize) -> &Commit { &self.entries[index].commit }

    /// Undos a commit by updating the history to the new head and returning the
    /// commit index.
    pub(super) fn undo(&mut self) -> Option<usize> {
        let prev_head = self.head;
        self.head = self.entries[prev_head].previous?;

        self.entries[self.head].hot_path = Some(prev_head);
        Some(prev_head)
    }

    /// Greedily redos on the "hot path", returning the commit index.
    pub(super) fn hot_redo(&mut self) -> Option<usize> {
        self.head = self.entries[self.head].hot_path?;
        Some(self.head)
    }
}
