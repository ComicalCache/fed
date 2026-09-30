use crossterm::event::KeyEvent;
use fancy_regex::Regex;
use piece_table::Slice;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    decorators::SearchDecorator,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::search::{command::Command, keymap},
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{DocId, StateLock, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct SearchKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
}

impl SearchKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    ) -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), state_lock, action_tx, mini_buffer_tx }
    }

    fn execute(&self, cmd: Command) {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let read_only = dse.read_only;
        drop(state);

        match cmd {
            Command::Move(motion) => {
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion,
                    move_anchor: false,
                });
            }
            Command::NextMatch => self.navigate_match(view, true),
            Command::PrevMatch => self.navigate_match(view, false),
            Command::CursorsBegin => self.cursors(view, true),
            Command::CursorsEnd => self.cursors(view, false),
            Command::Replace => {
                if read_only {
                    return;
                }

                self.replace(view, doc)
            }
            Command::Escape => Self::escape(self.state_lock.clone(), view, self.action_tx.clone()),
        }
    }

    fn navigate_match(&self, view: ViewId, forward: bool) {
        let Some((_, matches, offsets)) = self.search_state(view) else { return };
        let offset = offsets.first().copied().unwrap_or(0);

        let target = if forward {
            matches.iter().find(|m| m.0 > offset).unwrap_or(&matches[0]).0
        } else {
            matches.iter().rev().find(|m| m.0 < offset).unwrap_or(matches.last().unwrap()).0
        };

        let _ = self.action_tx.send(ActionCommand::RemoveCursors { view, offsets });
        let _ = self
            .action_tx
            .send(ActionCommand::CreateCursorsAtOffset { view, offsets: vec![target] });
    }

    fn cursors(&self, view: ViewId, front: bool) {
        let Some((_, matches, offsets)) = self.search_state(view) else { return };

        let _ = self.action_tx.send(ActionCommand::RemoveCursors { view, offsets });
        let offsets = matches.iter().map(|m| if front { m.0 } else { m.1 }).collect();
        let _ = self.action_tx.send(ActionCommand::CreateCursorsAtOffset { view, offsets });

        Self::escape(self.state_lock.clone(), view, self.action_tx.clone());
    }

    fn replace(&self, view: ViewId, doc: DocId) {
        let Some((query, matches, _)) = self.search_state(view) else { return };

        let (id_tx, _) = oneshot::channel();
        let (res_tx, res_rx) = oneshot::channel();
        let _ = self.mini_buffer_tx.send(MiniBufferCommand::Prompt {
            prompt: "Replace with: ".to_string(),
            initial_text: None,
            id_tx,
            res_tx,
        });

        tokio::spawn(Self::replace_completion(
            self.state_lock.clone(),
            view,
            doc,
            query,
            matches,
            res_rx,
            self.action_tx.clone(),
        ));
    }

    async fn replace_completion(
        state_lock: StateLock, view: ViewId, doc: DocId, query: String,
        matches: Vec<(usize, usize)>, res_rx: oneshot::Receiver<String>,
        action_tx: UnboundedSender<ActionCommand>,
    ) {
        let Ok(replacement) = res_rx.await else { return };
        let Ok(regex) = Regex::new(&query) else {
            // TODO: display error about regex.
            return;
        };

        let state = state_lock.read();
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut edits = Vec::new();
        for (start, end) in matches {
            let substring = dse.doc.data.slice(start..end);
            if let Ok(Some(caps)) = regex.captures(&substring) {
                let mut text = String::new();
                caps.expand(&replacement, &mut text);
                edits.push((start, end - start, text));
            }
        }
        drop(state);

        edits.sort_by_key(|e| std::cmp::Reverse(e.0));

        let _ = action_tx.send(ActionCommand::StartCommit { doc });

        for (offset, len, text) in edits {
            let _ = action_tx.send(ActionCommand::Remove { view, offset, len });
            if !text.is_empty() {
                let _ = action_tx.send(ActionCommand::InsertAt { view, text, offset });
            }
        }

        let _ = action_tx.send(ActionCommand::EndCommit { doc });

        Self::escape(state_lock, view, action_tx);
    }

    fn escape(state_lock: StateLock, view: ViewId, action_tx: UnboundedSender<ActionCommand>) {
        let mut state = state_lock.write();
        let Some(vse) = state.view_store.get_mut(&view) else {
            debug_panic!();
            return;
        };

        vse.decs.decorators.remove(&ViewStoreTypes::DecorationId::Search);

        drop(state);

        let _ = action_tx.send(ActionCommand::PopViewMode { view });
    }

    fn search_state(&self, view: ViewId) -> Option<(String, Vec<(usize, usize)>, Vec<usize>)> {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };
        let Some(provider) = vse.decs.decorators.get(&ViewStoreTypes::DecorationId::Search) else {
            debug_panic!();
            return None;
        };
        let Some(search_provider) = provider.any().downcast_ref::<SearchDecorator>() else {
            debug_panic!();
            return None;
        };

        if search_provider.matches.is_empty() {
            return None;
        }

        let query = search_provider.query.clone();
        let matches = search_provider.matches.clone();
        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();

        Some((query, matches, offsets))
    }
}

impl KeyInputHandler for SearchKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::SearchMode }

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

        if vse.mode() != ViewStoreTypes::Mode::Search {
            return false;
        }
        drop(state);

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                self.execute(cmd);
                self.pending_keys.clear();

                true
            }
            ParseResult::Prefix => true,
            ParseResult::Invalid => {
                self.pending_keys.clear();

                false
            }
        }
    }
}
