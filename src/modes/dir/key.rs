use std::path::PathBuf;

use crossterm::event::KeyEvent;
use piece_table::Slice;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    input_handler::{KeyInputHandler, KeyInputPriority},
    modes::{
        dir::{command::Command, keymap},
        search,
    },
    protocols::{action::ActionCmd, dir::DirCmd, mp::MpCmd, view::ViewCmd},
    state::{DirTypes, DocStoreTypes, State, ViewId, ViewStoreTypes},
    types::{KeyChord, Keymap, Motion, ParseResult},
    util,
};

pub struct DirKeyInput {
    keymap: Keymap<Command>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl DirKeyInput {
    pub fn new() -> Self {
        Self { keymap: keymap::keymap(), pending_keys: Vec::new(), last_view: None }
    }

    fn exec(&mut self, state: &State, cmd: Command) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else { return None };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return None;
        };

        match cmd {
            Command::Yank(_) | Command::Move(_) => self.motion(state, view, cmd),
            Command::YankLine => self.line_operation(state, view),
            Command::ScrollView(motion) => self.scroll_view(state, view, motion),
            Command::Jump => self.jump(view),
            Command::Create => self.create(state),
            Command::Rename => self.rename(state, view),
            Command::Delete => self.delete(state, view, false),
            Command::DeleteRecursive => self.delete(state, view, true),
            Command::Select => Some(vec![FCmd::Dir(DirCmd::Select)]),
            Command::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Command::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Command::Quit => Some(vec![FCmd::Quit]),
        }
    }

    fn motion(&self, state: &State, view: ViewId, cmd: Command) -> Option<Vec<FCmd>> {
        let motion = match cmd {
            Command::Move(motion) => motion,
            Command::Yank(motion) => motion,
            _ => unreachable!(),
        };

        if matches!(cmd, Command::Move(_)) {
            return Some(vec![FCmd::Action(ActionCmd::MoveCursors {
                view,
                motion,
                move_anchor: true,
            })]);
        }

        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let offsets = util::motion::offsets(motion, vse, dse);
        if offsets.is_empty() {
            return None;
        }

        let mut yanked = String::new();
        for &(start, end) in &offsets {
            let s = start.min(end);
            let e = start.max(end);
            yanked.push_str(&dse.doc.data.slice(s..e));

            // Newline to separate multi-cursor yanks.
            yanked.push('\n');
        }
        yanked.pop();

        if !yanked.is_empty()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(yanked);
        }

        Some(Vec::new())
    }

    fn line_operation(&self, state: &State, view: ViewId) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let mut lines: Vec<_> =
            vse.cursors.list.iter().map(|c| dse.doc.data.get_line_of_byte(c.offset)).collect();
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return None;
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

        Some(Vec::new())
    }

    fn scroll_view(&self, state: &State, view: ViewId, motion: Motion) -> Option<Vec<FCmd>> {
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };

        let mut scroll = *vse.scroll;
        match motion {
            Motion::Up => scroll.y = scroll.y.saturating_sub(1),
            Motion::Down => scroll.y = scroll.y.saturating_add(1),
            Motion::Left => scroll.x = scroll.x.saturating_sub(1),
            Motion::Right => scroll.x = scroll.x.saturating_add(1),
            _ => {}
        }

        Some(vec![FCmd::View(ViewCmd::ScrollTo { view, pos: scroll })])
    }

    fn jump(&self, view: ViewId) -> Option<Vec<FCmd>> {
        let pcallback = Box::new(move |state: &State, res: String| {
            if res.is_empty() {
                return;
            }

            let parts: Vec<&str> = res.split(':').collect();

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

            let _ = state.fcmd_tx.send(FCmd::Action(ActionCmd::MoveCursorToPos {
                view,
                pos: crate::types::Pos::new(x, y),
                move_anchor: true,
            }));
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "Jump to: ".to_string(),
            initial_text: None,
            pcallback,
            tx: None,
        })])
    }

    fn create(&self, state: &State) -> Option<Vec<FCmd>> {
        let mut initial_text = state.dir.pwd.display().to_string();

        if !initial_text.ends_with(std::path::MAIN_SEPARATOR) {
            initial_text.push(std::path::MAIN_SEPARATOR);
        }

        let pcallback = Box::new(move |state: &State, path: String| {
            let dir = path.ends_with(std::path::MAIN_SEPARATOR);
            let path = PathBuf::from(path);

            if !path.is_empty() {
                let _ = state.fcmd_tx.send(FCmd::Dir(DirCmd::Create { path, dir }));
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "New entry (trailing '/' for directory): ".to_string(),
            initial_text: Some(initial_text),
            pcallback,
            tx: None,
        })])
    }

    fn rename(&self, state: &State, view: ViewId) -> Option<Vec<FCmd>> {
        let Some(entry) = self.curr_entry(state, view) else { return None };
        if entry.path.file_name().is_none() {
            return None;
        }

        let old = entry.path.clone();
        let pcallback = Box::new(move |state: &State, new: String| {
            let new = util::path::normalize(new);

            if !new.is_empty() {
                let _ = state.fcmd_tx.send(FCmd::Dir(DirCmd::Rename { old, new }));
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "New path: ".to_string(),
            initial_text: Some(entry.path.display().to_string()),
            pcallback,
            tx: None,
        })])
    }

    fn delete(&self, state: &State, view: ViewId, recursive: bool) -> Option<Vec<FCmd>> {
        let Some(entry) = self.curr_entry(state, view) else { return None };
        if entry.path.file_name().is_none() {
            return None;
        }

        let path = entry.path.clone();
        let name = path.display().to_string();

        let prompt = if recursive {
            format!("Delete recursively '{}'? (y/N): ", name)
        } else {
            format!("Delete '{}'? (y/N): ", name)
        };

        let pcallback = Box::new(move |state: &State, res: String| {
            if res.to_lowercase() == "y" {
                let _ = state.fcmd_tx.send(FCmd::Dir(DirCmd::Delete { path, recursive }));
            }
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt { prompt, initial_text: None, pcallback, tx: None })])
    }

    fn curr_entry(&self, state: &State, view: ViewId) -> Option<DirTypes::Entry> {
        let Some((vse, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        let offset = vse.cursors.list.first().map(|c| c.offset).unwrap_or(0);
        let y = dse.doc.data.get_line_of_byte(offset);

        // Ignore the header and ".." entry.
        if y < 2 {
            return None;
        }

        state.dir.entries.get(y).cloned()
    }
}

impl KeyInputHandler for DirKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::DirMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return None;
        };
        let Some((_, dse)) =
            State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
        else {
            debug_panic!();
            return None;
        };

        if dse.mode != DocStoreTypes::Mode::Dir {
            return None;
        }

        if self.last_view != Some(view) {
            self.pending_keys.clear();

            self.last_view = Some(view);
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                // Always treat entering execute as consuming the key.
                let cmds = Some(self.exec(state, cmd).unwrap_or_else(|| Vec::new()));
                self.pending_keys.clear();

                cmds
            }
            ParseResult::Prefix => Some(Vec::new()),
            ParseResult::Invalid => {
                self.pending_keys.clear();

                Some(Vec::new())
            }
        }
    }
}
