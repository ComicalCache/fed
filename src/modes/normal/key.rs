use std::{path::PathBuf, time::Duration};

use crossterm::event::{KeyCode, KeyEvent};
use piece_table::Slice;
use tokio::sync::mpsc::UnboundedSender;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    fed::FCmd,
    modes::{
        common,
        normal::{cmd::Cmd, keymap},
        search,
    },
    protocols::{
        action::ActionCmd,
        dir::DirCmd,
        io::{IoFuture, IoProtocol},
        manage::ManageCmd,
        mp::MpCmd,
    },
    state::{DocId, State, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, Motion, ParseResult},
    util::{self, debug_panic},
};

pub struct NormalKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    replace: bool,

    last_view: Option<ViewId>,
}

impl NormalKeyInput {
    pub fn new() -> Self {
        Self { keymap: keymap::keymap(), pending_keys: Vec::new(), replace: false, last_view: None }
    }

    fn exec(&mut self, s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let Some(doc) = s.index.view_to_doc(view) else {
            debug_panic!();
            return None;
        };
        let Some(dse) = s.doc_store.get(&doc) else {
            debug_panic!();
            return None;
        };

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::Yank(motion) => common::yank(s, view, motion),
            Cmd::Delete(_) | Cmd::Change(_) => {
                if dse.read_only {
                    return None;
                }

                Self::motion(s, view, doc, cmd)
            }
            Cmd::YankLine => common::yank_line(s, view),
            Cmd::DeleteLine | Cmd::ChangeLine | Cmd::Indent | Cmd::Dedent => {
                if dse.read_only {
                    return None;
                }

                Self::line_operation(s, view, doc, cmd)
            }
            Cmd::ScrollView(motion) => common::scroll_view(s, view, motion),
            Cmd::Undo => {
                if dse.read_only {
                    return None;
                }

                Some(vec![FCmd::Action(ActionCmd::Undo { view })])
            }
            Cmd::HotRedo => {
                if dse.read_only {
                    return None;
                }

                Some(vec![FCmd::Action(ActionCmd::HotRedo { view })])
            }
            Cmd::Append | Cmd::AppendEndOfLine | Cmd::InsertLineBelow | Cmd::InsertLineAbove => {
                if dse.read_only {
                    return None;
                }

                Self::insert(view, doc, cmd)
            }
            Cmd::SwapLineDown | Cmd::SwapLineUp => {
                if dse.read_only {
                    return None;
                }

                Self::swap_operation(s, view, doc, cmd)
            }
            Cmd::Paste => {
                if !dse.read_only
                    && let Ok(mut clipboard) = arboard::Clipboard::new()
                    && let Ok(text) = clipboard.get_text()
                {
                    Some(vec![
                        FCmd::Action(ActionCmd::StartCommit { doc }),
                        FCmd::Action(ActionCmd::Insert { view, text }),
                        FCmd::Action(ActionCmd::EndCommit { doc }),
                    ])
                } else {
                    None
                }
            }
            Cmd::DeleteChar => {
                if dse.read_only {
                    return None;
                }

                Some(vec![
                    FCmd::Action(ActionCmd::StartCommit { doc }),
                    FCmd::Action(ActionCmd::Delete { view }),
                    FCmd::Action(ActionCmd::EndCommit { doc }),
                ])
            }
            Cmd::Replace => {
                if dse.read_only {
                    return None;
                }

                self.replace = true;

                None
            }
            Cmd::ReplaceChar(ch) => {
                if dse.read_only {
                    return None;
                }

                Self::replace_char(s, view, doc, ch)
            }
            Cmd::SaveFile => Self::save_file(s, doc),
            Cmd::Jump => common::jump(s, view),
            Cmd::SplitWindow(direction) => common::split_window(s, doc, direction),
            Cmd::FocusWindow(motion) => common::focus_window(s, motion),
            Cmd::EnterDirMode => {
                let Some(window) = s.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Dir(DirCmd::ReplaceWindow { window })])
            }
            Cmd::EnterManageMode => {
                let Some(window) = s.workspace.active_window else {
                    debug_panic!();
                    return None;
                };

                Some(vec![FCmd::Manage(ManageCmd::ReplaceWindow { window })])
            }
            Cmd::EnterInsertMode => {
                if dse.read_only {
                    return None;
                }

                Some(vec![
                    FCmd::Action(ActionCmd::StartCommit { doc }),
                    FCmd::Action(ActionCmd::PushViewMode {
                        view,
                        mode: ViewStoreTypes::Mode::Insert,
                    }),
                ])
            }
            Cmd::EnterVisualMode => Some(vec![FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Visual,
            })]),
            Cmd::EnterSearchMode => Some(search::util::start_search(view, doc, None)),
            Cmd::Quit => common::quit(s, view),
        }
    }

    fn motion(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let (Cmd::Delete(motion) | Cmd::Change(motion)) = cmd else { unreachable!() };

        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return None;
        };

        let offsets = util::motion::offsets(motion, vse, dse);
        if offsets.is_empty() {
            return None;
        }

        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];

        cmds.extend(common::remove_offsets(view, offsets));

        if matches!(cmd, Cmd::Change(_)) {
            cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Insert,
            }));
        } else if matches!(cmd, Cmd::Delete(_)) {
            cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
        }

        Some(cmds)
    }

    fn line_operation(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
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

        let mut cmds = Vec::new();
        match cmd {
            Cmd::DeleteLine => {
                for y in lines.into_iter().rev() {
                    let mut start = dse.doc.data.get_line_start_byte(y);
                    let end = dse.doc.data.get_line_end_byte(y);

                    // When deleting the last line, delete the newline from the
                    // previous line. This will break for Windows CRLF line
                    // endings.
                    if y + 1 == dse.doc.data.lines() {
                        start = start.saturating_sub(1);
                    }

                    cmds.push(FCmd::Action(ActionCmd::Remove {
                        view,
                        offset: start,
                        len: end - start,
                    }));
                }
            }
            Cmd::ChangeLine => {
                for y in lines.into_iter().rev() {
                    let start = dse.doc.data.get_line_start_byte(y);
                    let mut end = dse.doc.data.get_line_end_byte(y);

                    // Remove the newline of not the last line. This will break
                    // for Windows CRLF line endings.
                    if y + 1 != dse.doc.data.lines() {
                        end -= 1;
                    }

                    cmds.push(FCmd::Action(ActionCmd::Remove {
                        view,
                        offset: start,
                        len: end - start,
                    }));
                }

                cmds.push(FCmd::Action(ActionCmd::PushViewMode {
                    view,
                    mode: ViewStoreTypes::Mode::Insert,
                }));
            }
            Cmd::Indent => {
                // FIXME: option to convert tab to spaces.
                for y in lines.into_iter().rev() {
                    cmds.push(FCmd::Action(ActionCmd::InsertAt {
                        view,
                        text: "\t".to_string(),
                        offset: dse.doc.data.get_line_start_byte(y),
                    }));
                }
            }
            Cmd::Dedent => {
                // FIXME: option to also respect leading spaces.
                for y in lines.into_iter().rev() {
                    let start = dse.doc.data.get_line_start_byte(y);
                    let end = dse.doc.data.get_line_end_byte(y);

                    if dse.doc.data.slice(start..end).starts_with('\t') {
                        cmds.push(FCmd::Action(ActionCmd::Remove { view, offset: start, len: 1 }));
                    }
                }
            }
            _ => unreachable!(),
        }

        if cmds.is_empty() {
            return None;
        }

        cmds.insert(0, FCmd::Action(ActionCmd::StartCommit { doc }));
        if !matches!(cmd, Cmd::ChangeLine) {
            cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));
        }

        Some(cmds)
    }

    fn insert(view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];

        match cmd {
            Cmd::Append => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::Right,
                    move_anchor: true,
                }));
            }
            Cmd::AppendEndOfLine => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::EndOfLine,
                    move_anchor: true,
                }));
            }
            Cmd::InsertLineBelow => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::EndOfLine,
                    move_anchor: true,
                }));
                cmds.push(FCmd::Action(ActionCmd::Insert { view, text: "\n".to_string() }));
            }
            Cmd::InsertLineAbove => {
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::BeginningOfLine,
                    move_anchor: true,
                }));
                cmds.push(FCmd::Action(ActionCmd::Insert { view, text: "\n".to_string() }));
                cmds.push(FCmd::Action(ActionCmd::MoveCursors {
                    view,
                    motion: Motion::Left,
                    move_anchor: true,
                }));
            }
            _ => unreachable!(),
        }

        cmds.push(FCmd::Action(ActionCmd::PushViewMode {
            view,
            mode: ViewStoreTypes::Mode::Insert,
        }));

        Some(cmds)
    }

    fn swap_operation(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return None;
        };

        let lines: Vec<_> =
            vse.cursors.list.iter().map(|c| dse.doc.data.get_line_of_byte(c.offset)).collect();

        common::swap_lines(s, view, doc, lines, matches!(cmd, Cmd::SwapLineDown))
    }

    fn replace_char(s: &State, view: ViewId, doc: DocId, ch: char) -> Option<Vec<FCmd>> {
        let Some((vse, dse)) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view)
        else {
            debug_panic!();
            return None;
        };

        let mut edits = Vec::new();
        for cursor in &vse.cursors.list {
            let end = dse.doc.data.get_line_end_byte(dse.doc.data.get_line_of_byte(cursor.offset));

            if cursor.offset < end {
                let text = dse.doc.data.slice(cursor.offset..end);

                // Disallow replacing newline characters.This will break for
                // Windows CRLF line endings.
                if text.starts_with('\n') {
                    continue;
                }

                let len = text.graphemes(true).next().map_or(1, str::len);
                edits.push((cursor.offset, len));
            }
        }

        if edits.is_empty() {
            return None;
        }
        edits.sort_unstable_by_key(|&(o, _)| std::cmp::Reverse(o));
        edits.dedup_by_key(|&mut (o, _)| o);

        let mut cmds = vec![FCmd::Action(ActionCmd::StartCommit { doc })];
        for (offset, len) in edits {
            cmds.push(FCmd::Action(ActionCmd::Remove { view, offset, len }));
            cmds.push(FCmd::Action(ActionCmd::InsertAt { view, text: ch.to_string(), offset }));
        }
        cmds.push(FCmd::Action(ActionCmd::MoveCursors {
            view,
            motion: Motion::Left,
            move_anchor: true,
        }));
        cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));

        Some(cmds)
    }

    fn save_file(s: &State, doc: DocId) -> Option<Vec<FCmd>> {
        let Some(dse) = s.doc_store.get(&doc) else {
            debug_panic!();
            return None;
        };

        let path = dse.doc.path.as_ref().map_or_else(
            || {
                format!(
                    "{}{}",
                    std::env::current_dir().unwrap_or_default().display(),
                    std::path::MAIN_SEPARATOR
                )
            },
            |p| p.display().to_string(),
        );

        let pcallback = Box::new(move |s: &mut State, res: String| {
            if res.is_empty() {
                return;
            }

            let Some(dse) = s.doc_store.get(&doc) else {
                debug_panic!();
                return;
            };

            let path = util::path::normalize(res);
            let data = dse.doc.data.slice(0..dse.doc.data.len());

            let cb_path = path.clone();
            let async_fcmd_tx = s.async_fcmd_tx.clone();
            let wcallback = Box::new(move |res| -> IoFuture {
                Box::pin(
                    async move { Self::save_file_completion(async_fcmd_tx, res, doc, cb_path) },
                )
            });

            IoProtocol::write(s, path, data, wcallback);
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "Save file: ".to_string(),
            initial_text: if path.is_empty() { None } else { Some(path) },
            pcallback,
            tx: None,
        })])
    }

    fn save_file_completion(
        async_fcmd_tx: UnboundedSender<FCmd>, res: Result<(), String>, doc: DocId, path: PathBuf,
    ) {
        match res {
            Ok(()) => {
                let _ = async_fcmd_tx
                    .send(FCmd::Action(ActionCmd::SetDocPath { doc, path: Some(path) }));
                let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::Saved { doc }));

                util::mp::async_message(
                    async_fcmd_tx,
                    "File saved".to_string(),
                    Duration::from_secs(3),
                );
            }
            Err(err) => {
                util::mp::async_message(
                    async_fcmd_tx,
                    format!("Error saving file: {err}"),
                    Duration::from_secs(3),
                );
            }
        }
    }
}

impl KeyInputHandler for NormalKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::NormalMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let Some(vse) = s.view_store.get(&view) else {
            debug_panic!();
            return None;
        };

        if vse.mode() != ViewStoreTypes::Mode::Normal {
            return None;
        }

        if self.last_view != Some(view) {
            self.replace = false;
            self.pending_keys.clear();

            self.last_view = Some(view);
        }

        if self.replace {
            let cmds = if let KeyCode::Char(ch) = event.code {
                // Always treat entering execute as consuming the key.
                Some(self.exec(s, Cmd::ReplaceChar(ch)).unwrap_or_default())
            } else {
                None
            };

            self.replace = false;
            self.pending_keys.clear();

            return cmds;
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                // Always treat entering execute as consuming the key.
                let cmds = Some(self.exec(s, cmd).unwrap_or_default());
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
