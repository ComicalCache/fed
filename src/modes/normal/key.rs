use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use piece_table::Slice;
use tokio::sync::mpsc::UnboundedSender;
use unicode_segmentation::UnicodeSegmentation;

use crate::fed::FCmd;
use crate::modes::common;
use crate::modes::normal::cmd::Cmd;
use crate::modes::normal::keymap;
use crate::modes::picker::entries::ScanEntry;
use crate::modes::picker::entries::ThemeEntry;
use crate::modes::search;
use crate::protocols::action::ActionCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::dir::DirCmd;
use crate::protocols::dir::DirProtocol;
use crate::protocols::doc_view::DocViewCmd;
use crate::protocols::doc_view::DocViewProtocol;
use crate::protocols::io::IoFuture;
use crate::protocols::io::IoProtocol;
use crate::protocols::manage::ManageCmd;
use crate::protocols::mp::MpCmd;
use crate::protocols::picker::PickerCmd;
use crate::protocols::picker::PickerEntry;
use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::KeyChord;
use crate::types::KeyInputHandler;
use crate::types::KeyInputPriority;
use crate::types::Keymap;
use crate::types::Motion;
use crate::types::ParseResult;
use crate::types::Theme;
use crate::util;

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
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get(&doc).unwrap();

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
            Cmd::ScrollView(motion) => Some(common::scroll_view(s, view, motion)),
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

                Some(Self::insert(view, doc, cmd))
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
                    Some(common::paste(s, view, &text))
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
            Cmd::SaveFile => Some(Self::save_file(s, doc)),
            Cmd::FindFile => Some(Self::find_file(s)),
            Cmd::SelectTheme => Some(Self::select_theme()),
            Cmd::Jump => common::jump(s, view),
            Cmd::SplitWindow(direction) => common::split_window(s, doc, direction),
            Cmd::FocusWindow(motion) => common::focus_window(s, motion),
            Cmd::EnterDirMode => {
                let window = s.workspace.active_window?;
                Some(vec![FCmd::Dir(DirCmd::ReplaceWindow { window, path: None })])
            }
            Cmd::EnterManageMode => {
                let window = s.workspace.active_window?;
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

        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

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
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

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

    fn insert(view: ViewId, doc: DocId, cmd: Cmd) -> Vec<FCmd> {
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

        cmds
    }

    fn swap_operation(s: &State, view: ViewId, doc: DocId, cmd: Cmd) -> Option<Vec<FCmd>> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        let lines: Vec<_> =
            vse.cursors.list.iter().map(|c| dse.doc.data.get_line_of_byte(c.offset)).collect();

        common::swap_lines(s, view, doc, lines, matches!(cmd, Cmd::SwapLineDown))
    }

    fn replace_char(s: &State, view: ViewId, doc: DocId, ch: char) -> Option<Vec<FCmd>> {
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

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

    fn find_file(s: &State) -> Vec<FCmd> {
        let window = s.workspace.active_window.unwrap();
        let entries =
            s.p.io
                .scan_list
                .iter()
                .cloned()
                .map(|entry| Box::new(entry) as Box<dyn PickerEntry>)
                .collect();

        let callback = Box::new(move |s: &mut State, selected: Option<Box<dyn PickerEntry>>| {
            if s.workspace.get_rect(window).is_none() {
                return;
            }

            let Some(selected) = selected else { return };

            let entry = selected.into_any().downcast::<ScanEntry>().unwrap();
            ActionProtocol::set_active_window(s, Some(window));

            if entry.dir {
                DirProtocol::exec(s, DirCmd::ReplaceWindow { window, path: Some(entry.path) });
            } else {
                DocViewProtocol::exec(
                    s,
                    DocViewCmd::OpenFile { window, path: entry.path, force: true },
                );
            }
        });

        vec![FCmd::Picker(PickerCmd::Open {
            width: s.workspace.rect().width * 9 / 10,
            height: s.workspace.rect().height * 9 / 10,
            entries,
            callback,
        })]
    }

    fn select_theme() -> Vec<FCmd> {
        let entries: Vec<Box<dyn PickerEntry>> = vec![
            Box::new(ThemeEntry { name: "Dark", theme: Theme::dark() }),
            Box::new(ThemeEntry { name: "Light", theme: Theme::light() }),
        ];

        let callback = Box::new(|s: &mut State, selected: Option<Box<dyn PickerEntry>>| {
            let Some(selected) = selected else { return };

            let entry = selected.into_any().downcast::<ThemeEntry>().unwrap();
            s.theme = entry.theme;
        });

        vec![FCmd::Picker(PickerCmd::Open { width: 24, height: 9, entries, callback })]
    }

    fn save_file(s: &State, doc: DocId) -> Vec<FCmd> {
        let dse = s.doc_store.get(&doc).unwrap();

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

            let dse = s.doc_store.get(&doc).unwrap();

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

        vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "Save file: ".to_string(),
            initial_text: if path.is_empty() { None } else { Some(path) },
            pcallback,
            tx: None,
        })]
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
        let vse = s.view_store.get(&view).unwrap();

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
