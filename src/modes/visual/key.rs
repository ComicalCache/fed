use crossterm::event::KeyEvent;
use piece_table::Slice;
use tokio::sync::mpsc::UnboundedSender;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::{
        search,
        visual::{command::Command, keymap},
    },
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{DocId, StateLock, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyNode, Keymap},
};

pub struct VisualKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
}

impl VisualKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    ) -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), state_lock, action_tx, mini_buffer_tx }
    }

    fn execute(&self, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        drop(state);

        match cmd {
            Command::Move(motion) => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion: *motion,
                    move_anchor: false,
                });
            }
            Command::EnterSearchMode => self.enter_search_mode(view, doc),
            Command::Escape => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Normal });
            }
            Command::Delete | Command::Change | Command::Yank => self.operator(view, doc, cmd),
        }
    }

    fn enter_search_mode(&self, view: ViewId, doc: DocId) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut offsets = Vec::new();
        for cursor in &vse.cursors.list {
            let start = cursor.anchor.min(cursor.offset);
            let mut end = cursor.anchor.max(cursor.offset);

            let text = dse.doc.data.slice(end..dse.doc.data.len());
            if let Some(grapheme) = text.graphemes(true).next() {
                end += grapheme.len();
            }

            offsets.push((start, end));
        }
        drop(state);

        offsets.sort_by_key(|&(s, _)| s);

        let mut bounds: Vec<(usize, usize)> = Vec::new();
        for (start, end) in offsets {
            if let Some(last) = bounds.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }

            bounds.push((start, end));
        }

        search::util::start_search(
            self.state_lock.clone(),
            view,
            doc,
            Some(bounds),
            self.mini_buffer_tx.clone(),
        );
    }

    fn operator(&self, view: ViewId, doc: DocId, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut offsets = Vec::new();
        for cursor in &vse.cursors.list {
            let start = cursor.anchor.min(cursor.offset);
            let mut end = cursor.anchor.max(cursor.offset);

            let text = dse.doc.data.slice(end..dse.doc.data.len());
            if let Some(grapheme) = text.graphemes(true).next() {
                end += grapheme.len();
            }

            offsets.push((start, end));
        }
        drop(state);

        if matches!(cmd, Command::Yank) {
            let state = self.state_lock.read();
            let Some(dse) = state.doc_store.get(&doc) else {
                debug_panic!();
                return;
            };

            let mut yanked = String::new();
            for &(start, end) in &offsets {
                yanked.push_str(&dse.doc.data.slice(start..end));

                // Newline to separate multi-cursor yanks.
                yanked.push('\n');
            }
            drop(state);

            yanked.pop();

            if !yanked.is_empty()
                && let Ok(mut clipboard) = arboard::Clipboard::new()
            {
                let _ = clipboard.set_text(yanked);
            }

            let _ = self
                .action_tx
                .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Normal });

            return;
        }

        offsets.sort_by_key(|&(s, _)| s);

        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in offsets {
            if let Some(last) = merged.last_mut()
                && start <= last.1
            {
                last.1 = last.1.max(end);
                continue;
            }

            merged.push((start, end));
        }
        merged.sort_by_key(|&(s, _)| std::cmp::Reverse(s));

        if matches!(cmd, Command::Change) || matches!(cmd, Command::Delete) {
            let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
        }

        for (start, end) in merged {
            if end > start {
                let _ = self.action_tx.send(ActionCommand::Remove {
                    view,
                    offset: start,
                    len: end - start,
                });
            }
        }

        if matches!(cmd, Command::Change) {
            let _ = self
                .action_tx
                .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
        } else if matches!(cmd, Command::Delete) {
            let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
            let _ = self
                .action_tx
                .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Normal });
        }
    }
}

impl KeyInputHandler for VisualKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::VisualMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return false;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return false;
        };

        if vse.mode != ViewStoreTypes::Mode::Visual {
            return false;
        }
        drop(state);

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        let mut curr = &self.keymap.root;
        let mut target = None;
        for (idx, key_chord) in self.pending_keys.iter().enumerate() {
            if let Some(node) = curr.get(key_chord) {
                if idx == self.pending_keys.len() - 1 {
                    target = Some(node);
                } else if let KeyNode::Prefix(next_map) = node {
                    curr = next_map;
                } else {
                    debug_panic!();
                }
            } else {
                // Invalid sequence.
                self.pending_keys.clear();

                return false;
            }
        }

        match target {
            Some(KeyNode::Leaf(cmd)) => {
                self.execute(cmd);
                self.pending_keys.clear();

                true
            }
            Some(KeyNode::Prefix(_)) => true,
            None => {
                self.pending_keys.clear();

                false
            }
        }
    }
}
