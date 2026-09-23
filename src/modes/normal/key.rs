use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::normal::command::Command,
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{StateLock, ViewStoreTypes},
    types::{Direction, KeyChord, KeyNode, Keymap},
};

pub struct NormalKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    quit_tx: UnboundedSender<()>,
}

impl NormalKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>, quit_tx: UnboundedSender<()>,
    ) -> Self {
        let mut keymap = Keymap::new();

        keymap.bind(
            &[KeyChord::new(KeyCode::Char('h'), KeyModifiers::empty())],
            Command::Move(Direction::Left),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('j'), KeyModifiers::empty())],
            Command::Move(Direction::Down),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('k'), KeyModifiers::empty())],
            Command::Move(Direction::Up),
        );
        keymap.bind(
            &[KeyChord::new(KeyCode::Char('l'), KeyModifiers::empty())],
            Command::Move(Direction::Right),
        );

        keymap.bind(
            &[KeyChord::new(KeyCode::Char('i'), KeyModifiers::empty())],
            Command::EnterInsertMode,
        );

        keymap.bind(&[KeyChord::new(KeyCode::Char('q'), KeyModifiers::CONTROL)], Command::Quit);
        keymap
            .bind(&[KeyChord::new(KeyCode::Char('T'), KeyModifiers::SHIFT)], Command::TestMessage);
        keymap.bind(&[KeyChord::new(KeyCode::Char('P'), KeyModifiers::SHIFT)], Command::TestPrompt);

        Self { state_lock, action_tx, mini_buffer_tx, quit_tx, keymap, pending_keys: Vec::new() }
    }

    fn execute(&self, cmd: &Command) {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else { return };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return;
        };
        drop(state);

        match cmd {
            Command::Move(direction) => {
                let _ =
                    self.action_tx.send(ActionCommand::MoveCursors { view, direction: *direction });
            }
            Command::EnterInsertMode => {
                let _ = self.action_tx.send(ActionCommand::StartCommit { doc });
                let _ = self
                    .action_tx
                    .send(ActionCommand::SetViewMode { view, mode: ViewStoreTypes::Mode::Insert });
            }
            Command::Quit => {
                let _ = self.quit_tx.send(());
            }
            Command::TestMessage => {
                let (tx, rx) = oneshot::channel();
                let _ = self
                    .mini_buffer_tx
                    .send(MiniBufferCommand::Message { message: "Hello, world!".to_string(), tx });

                let tx_clone = self.mini_buffer_tx.clone();
                tokio::spawn(async move {
                    let Ok(id) = rx.await else { return };

                    tokio::time::sleep(Duration::from_secs(3)).await;

                    let _ = tx_clone.send(MiniBufferCommand::Close { id });
                });
            }
            Command::TestPrompt => {
                let (id_tx, _) = oneshot::channel();
                let (res_tx, res_rx) = oneshot::channel();
                let _ = self.mini_buffer_tx.send(MiniBufferCommand::Prompt {
                    prompt: "Hello, prompt: ".to_string(),
                    id_tx,
                    res_tx,
                });

                let mini_buffer_tx = self.mini_buffer_tx.clone();
                tokio::spawn(async move {
                    let Ok(res) = res_rx.await else { return };

                    let (tx, rx) = oneshot::channel();
                    let _ = mini_buffer_tx.send(MiniBufferCommand::Message {
                        message: format!("You typed: '{res}'"),
                        tx,
                    });

                    let tx_clone = mini_buffer_tx.clone();
                    tokio::spawn(async move {
                        let Ok(id) = rx.await else { return };

                        tokio::time::sleep(Duration::from_secs(3)).await;

                        let _ = tx_clone.send(MiniBufferCommand::Close { id });
                    });
                });
            }
        }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

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

        if vse.mode != ViewStoreTypes::Mode::Normal {
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
