use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::insert::command::Command,
    protocols::action::ActionCommand,
    state::{StateLock, ViewStoreTypes},
    types::{Direction, KeyChord, KeyNode, Keymap},
};

pub struct InsertKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
}

impl InsertKeyInput {
    pub fn new(state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>) -> Self {
        let mut keymap = Keymap::new();

        keymap.bind(
            &[KeyChord::new(KeyCode::Left, KeyModifiers::empty())],
            Command::Move(Direction::Left),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Right, KeyModifiers::empty())],
            Command::Move(Direction::Right),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Up, KeyModifiers::empty())],
            Command::Move(Direction::Up),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Down, KeyModifiers::empty())],
            Command::Move(Direction::Down),
        );

        keymap
            .bind(&[KeyChord::new(KeyCode::Backspace, KeyModifiers::empty())], Command::Backspace);
        keymap.bind(&[KeyChord::new(KeyCode::Delete, KeyModifiers::empty())], Command::Delete);

        keymap.bind(
            &[KeyChord::new(KeyCode::Enter, KeyModifiers::empty())],
            Command::Input("\n".to_string()),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Tab, KeyModifiers::empty())],
            Command::Input("\t".to_string()),
        );
        keymap.bind(&[KeyChord::new(KeyCode::Esc, KeyModifiers::empty())], Command::Escape);

        Self { state_lock, action_tx, keymap, pending_keys: Vec::new() }
    }

    fn execute(&self, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else { return };
        let Some(doc) = state.index.view_to_doc(view) else { return };
        drop(state);

        match cmd {
            Command::Move(direction) => {
                let _ =
                    self.action_tx.send(ActionCommand::MoveCursors { view, direction: *direction });
            }
            Command::Backspace => {
                let _ = self.action_tx.send(ActionCommand::Backspace { view });
            }
            Command::Delete => {
                let _ = self.action_tx.send(ActionCommand::Delete { view });
            }
            Command::Escape => {
                let _ = self.action_tx.send(ActionCommand::EndCommit { doc });
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Normal });
            }
            Command::Input(text) => {
                let _ = self.action_tx.send(ActionCommand::Insert { view, text: text.clone() });
            }
        }
    }
}

impl KeyInputHandler for InsertKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::InsertMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // Click active view, just abort.
            return false;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return false;
        };

        if vse.mode != ViewStoreTypes::Mode::Insert {
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
                    self.execute(&Command::Input(format!("<{modifiers}-{ch}>")));
                } else {
                    self.execute(&Command::Input(ch.to_string()));
                }

                return true;
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
