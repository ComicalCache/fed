use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::insert::{command::Command, keymap},
    protocols::action::ActionCommand,
    state::{DocId, State, StateLock, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, ParseResult},
};

pub struct InsertKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    whitespace: bool,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
}

impl InsertKeyInput {
    pub fn new(state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>) -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), whitespace: false, state_lock, action_tx }
    }

    fn execute(&mut self, cmd: Command) {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // Click active view, just abort.
            return;
        };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        drop(state);

        match cmd {
            Command::Input(text) => self.input(view, doc, text),
            Command::Move(motion) => {
                self.whitespace = false;
                let _ = self.action_tx.send(ActionCommand::MoveCursors {
                    view,
                    motion,
                    move_anchor: true,
                });
            }
            Command::Backspace => {
                self.whitespace = false;
                let _ = self.action_tx.send(ActionCommand::Backspace { view });
            }
            Command::Delete => {
                self.whitespace = false;
                let _ = self.action_tx.send(ActionCommand::Delete { view });
            }
            Command::Escape => {
                self.whitespace = false;
                let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
                let _ = self.action_tx.send(ActionCommand::PopViewMode { view });
            }
        }
    }

    fn input(&mut self, view: ViewId, doc: DocId, text: String) {
        let whitespace = text.chars().all(|c| c.is_whitespace());
        if whitespace && !self.whitespace {
            let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
            let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
        }
        self.whitespace = whitespace;

        let _ = self.action_tx.send(ActionCommand::Insert { view, text: text.to_string() });
    }
}

impl KeyInputHandler for InsertKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::InsertMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return false;
        };
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return false;
        };

        if vse.mode() != ViewStoreTypes::Mode::Insert {
            return false;
        }
        if dse.read_only {
            debug_panic!();
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

                // If it's a character input, insert it.
                let KeyCode::Char(ch) = event.code else { return false };

                let modifiers = event
                    .modifiers
                    .iter_names()
                    .filter_map(|(_, m)| match m {
                        KeyModifiers::ALT => Some("A"),
                        KeyModifiers::CONTROL => Some("C"),
                        KeyModifiers::META => Some("M"),
                        KeyModifiers::SUPER => Some("S"),
                        KeyModifiers::HYPER => Some("H"),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("-");

                if !modifiers.is_empty() {
                    self.execute(Command::Input(format!("<{modifiers}-{ch}>")));
                } else {
                    self.execute(Command::Input(ch.to_string()));
                }

                true
            }
        }
    }
}
