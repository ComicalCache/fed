use crossterm::event::KeyEvent;
use piece_table::Slice;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    input::{KeyInputHandler, priorities::KeyInputPriority},
    modes::{
        dir::{command::Command, keymap},
        search,
    },
    protocols::{
        action::ActionCommand, dir::DirCommand, mini_buffer::MiniBufferCommand, view::ViewCommand,
    },
    state::{DocId, DocStoreTypes, State, StateLock, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, Motion, ParseResult},
    util,
};

pub struct DirKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
    state_lock: StateLock,

    action_tx: UnboundedSender<ActionCommand>,
    mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
    quit_tx: UnboundedSender<()>,
    view_tx: UnboundedSender<ViewCommand>,
    dir_tx: UnboundedSender<DirCommand>,
}

impl DirKeyInput {
    pub fn new(
        state_lock: StateLock, action_tx: UnboundedSender<ActionCommand>,
        mini_buffer_tx: UnboundedSender<MiniBufferCommand>, quit_tx: UnboundedSender<()>,
        view_tx: UnboundedSender<ViewCommand>, dir_tx: UnboundedSender<DirCommand>,
    ) -> Self {
        Self {
            keymap: keymap::keymap(),
            pending_keys: Vec::new(),
            last_view: None,
            state_lock,
            action_tx,
            mini_buffer_tx,
            quit_tx,
            view_tx,
            dir_tx,
        }
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
            Command::Yank(_) | Command::Move(_) => self.motion(view, doc, cmd),
            Command::YankLine => self.line_operation(view, doc),
            Command::ScrollView(motion) => self.scroll_view(view, motion),
            Command::Jump => self.jump(view),
            Command::Select => {
                let _ = self.dir_tx.send(DirCommand::Select);
            }
            Command::EnterVisualMode => {
                let _ = self
                    .action_tx
                    .send(ActionCommand::PushViewMode { view, mode: ViewStoreTypes::Mode::Visual });
            }
            Command::EnterSearchMode => search::util::start_search(
                self.state_lock.clone(),
                view,
                doc,
                None,
                self.action_tx.clone(),
                self.mini_buffer_tx.clone(),
            ),
            Command::Quit => {
                let _ = self.quit_tx.send(());
            }
        }
    }

    fn motion(&self, view: ViewId, doc: DocId, cmd: Command) {
        let motion = match cmd {
            Command::Move(motion) => motion,
            Command::Yank(motion) => motion,
            _ => unreachable!(),
        };

        if matches!(cmd, Command::Move(_)) {
            let _ =
                self.action_tx.send(ActionCommand::MoveCursors { view, motion, move_anchor: true });
            return;
        }

        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let offsets = util::motion_offsets(motion, vse, dse);
        if offsets.is_empty() {
            return;
        }
        drop(state);

        let state = self.state_lock.read();
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut yanked = String::new();
        for &(start, end) in &offsets {
            let s = start.min(end);
            let e = start.max(end);
            yanked.push_str(&dse.doc.data.slice(s..e));

            // Newline to separate multi-cursor yanks.
            yanked.push('\n');
        }
        yanked.pop();

        drop(state);

        if !yanked.is_empty()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(yanked);
        }
    }

    fn line_operation(&self, view: ViewId, doc: DocId) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        let mut lines: Vec<_> =
            vse.cursors.list.iter().map(|c| dse.doc.data.get_line_of_byte(c.offset)).collect();
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return;
        }

        let mut yanked = String::new();
        for &y in &lines {
            let start = dse.doc.data.get_line_start_byte(y);
            let mut end = dse.doc.data.get_line_end_byte(y);

            // Remove the newline of not the last line. This will break
            // for Windows CRLF line endings.
            if y + 1 != dse.doc.data.lines() {
                end -= 1;
            }

            yanked.push_str(&dse.doc.data.slice(start..end));

            // Newline to separate multi-cursor yanks.
            yanked.push('\n');
        }
        yanked.pop();

        if !yanked.is_empty()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(yanked);
        }

        drop(state);
    }

    fn scroll_view(&self, view: ViewId, motion: Motion) {
        let state = self.state_lock.read();
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return;
        };

        let mut scroll = *vse.scroll;
        drop(state);

        match motion {
            Motion::Up => scroll.y = scroll.y.saturating_sub(1),
            Motion::Down => scroll.y = scroll.y.saturating_add(1),
            Motion::Left => scroll.x = scroll.x.saturating_sub(1),
            Motion::Right => scroll.x = scroll.x.saturating_add(1),
            _ => {}
        }

        let _ = self.view_tx.send(ViewCommand::ScrollTo { view, pos: scroll });
    }

    fn jump(&self, view: ViewId) {
        let (id_tx, _) = oneshot::channel();
        let (res_tx, res_rx) = oneshot::channel();

        let _ = self.mini_buffer_tx.send(MiniBufferCommand::Prompt {
            prompt: "Jump to: ".to_string(),
            initial_text: None,
            id_tx,
            res_tx,
        });

        tokio::spawn(Self::jump_completion(
            self.state_lock.clone(),
            view,
            res_rx,
            self.action_tx.clone(),
        ));
    }

    async fn jump_completion(
        state_lock: StateLock, view: ViewId, res_rx: oneshot::Receiver<String>,
        action_tx: UnboundedSender<ActionCommand>,
    ) {
        let Ok(res) = res_rx.await else { return };
        if res.is_empty() {
            return;
        }

        let parts: Vec<&str> = res.split(':').collect();

        let state = state_lock.read();
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return;
        };
        let Some(cursor) = vse.cursors.list.first() else {
            return;
        };

        let pos = util::offset_to_pos(cursor.offset, vse, dse);
        drop(state);

        let mut x = pos.x;
        let mut y = pos.y;

        if parts.len() > 0 && !parts[0].is_empty() {
            if let Ok(val) = parts[0].parse::<usize>() {
                y = val.saturating_sub(1);
            }
        }
        if parts.len() > 1 && !parts[1].is_empty() {
            if let Ok(val) = parts[1].parse::<usize>() {
                x = val.saturating_sub(1);
            }
        }

        let _ = action_tx.send(ActionCommand::MoveCursorToPos {
            view,
            pos: crate::types::Pos::new(x, y),
            move_anchor: true,
        });
    }
}

impl KeyInputHandler for DirKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::DirMode }

    fn key(&mut self, event: &KeyEvent) -> bool {
        let state = self.state_lock.read();
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return false;
        };
        let Some((_, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return false;
        };

        if dse.mode != DocStoreTypes::Mode::Dir {
            return false;
        }
        drop(state);

        if self.last_view != Some(view) {
            self.pending_keys.clear();

            self.last_view = Some(view);
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                self.execute(cmd);
                self.pending_keys.clear();
            }
            ParseResult::Prefix => {}
            ParseResult::Invalid => {
                self.pending_keys.clear();
            }
        }

        true
    }
}
