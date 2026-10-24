use std::ops::Range;

use crate::piece_table::Piece;

#[derive(Clone, Copy)]
pub enum Kind {
    Deletion,
    Insertion,
}

pub struct Change {
    /// Position of the Piece in the `PieceTable` at the time of changing.
    pub pos: usize,
    pub piece: Piece,
    pub kind: Kind,
}

impl Change {
    pub const fn new(pos: usize, piece: Piece, kind: Kind) -> Self { Self { pos, piece, kind } }
}

pub struct LogicalChange {
    pub pos: usize,
    pub kind: Kind,
    pub length: usize,

    pub changes: Range<usize>,
}

pub struct Commit {
    pub changes: Vec<Change>,
    pub logical_changes: Vec<LogicalChange>,
}

impl Commit {
    pub const fn new() -> Self { Self { changes: Vec::new(), logical_changes: Vec::new() } }

    pub fn add_change(&mut self, pos: usize, piece: Piece, kind: Kind) {
        self.changes.push(Change::new(pos, piece, kind));
    }

    pub fn add_logical_change(&mut self, pos: usize, kind: Kind, length: usize) {
        let start = self.logical_changes.last().map_or(0, |c| c.changes.end);
        self.logical_changes.push(LogicalChange {
            pos,
            kind,
            length,
            changes: start..self.changes.len(),
        });
    }
}

pub struct Entry {
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
pub struct History {
    changes: Vec<Entry>,
    head: usize,
}

impl History {
    pub fn new(commit: Commit) -> Self { Self { changes: vec![Entry::new(commit)], head: 0 } }

    /// Updates the history and adds the latest commit.
    pub fn save(&mut self, commit: Commit) {
        let prev_head = self.head;
        self.head = self.changes.len();

        self.changes[prev_head].next.push(self.head);

        let mut entry = Entry::new(commit);
        entry.previous = Some(prev_head);
        self.changes.push(entry);
    }

    pub fn commit(&self, index: usize) -> &Commit { &self.changes[index].commit }

    /// Undos a commit by updating the history to the new head and returning the
    /// commit index.
    pub fn undo(&mut self) -> Option<usize> {
        let prev_head = self.head;
        self.head = self.changes[prev_head].previous?;

        self.changes[self.head].hot_path = Some(prev_head);
        Some(prev_head)
    }

    /// Greedily redos on the "hot path", returning the commit index.
    pub fn hot_redo(&mut self) -> Option<usize> {
        self.head = self.changes[self.head].hot_path?;
        Some(self.head)
    }
}
