use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

use crate::fed::FCmd;
use crate::modes::picker::entries::ThemeEntry;
use crate::piece_table::Chunks;
use crate::protocols::action::ActionCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::dir::DirCmd;
use crate::protocols::dir::DirProtocol;
use crate::protocols::doc_view::DocViewCmd;
use crate::protocols::doc_view::DocViewProtocol;
use crate::protocols::io::IoFuture;
use crate::protocols::io::IoProtocol;
use crate::protocols::io::ScanEntry;
use crate::protocols::mp::MpCmd;
use crate::protocols::picker::PickerCmd;
use crate::protocols::picker::PickerEntry;
use crate::protocols::view::ViewCmd;
use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::Motion;
use crate::types::Pos;
use crate::types::RectSplit;
use crate::types::Theme;
use crate::types::render::WindowId;
use crate::util;

pub fn mouse(s: &State, window: WindowId, view: ViewId, event: MouseEvent) -> Vec<FCmd> {
    match event.kind {
        MouseEventKind::ScrollUp => return scroll_view(s, view, Motion::Up),
        MouseEventKind::ScrollDown => return scroll_view(s, view, Motion::Down),
        MouseEventKind::ScrollLeft => return scroll_view(s, view, Motion::Left),
        MouseEventKind::ScrollRight => return scroll_view(s, view, Motion::Right),
        MouseEventKind::Down(MouseButton::Left) => {}
        _ => return Vec::new(),
    }

    let rect = s.workspace.get_rect(window).unwrap();
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let mut cmds = vec![FCmd::Action(ActionCmd::SetActiveWindow { window: Some(window) })];

    let content = vse.layout(dse).content;
    let mut pos = Pos::from((event.column, event.row)).saturating_sub(rect.pos);
    if !content.contains(pos) {
        return cmds;
    }

    pos = pos.saturating_sub(content.pos) + *vse.scroll;

    if event.modifiers.contains(KeyModifiers::ALT) {
        match dse.max_cursors {
            Some(max) => {
                if vse.cursors.list.len() < max {
                    cmds.push(FCmd::Action(ActionCmd::CreateCursorAtPos { view, pos }));
                }
            }
            None => cmds.push(FCmd::Action(ActionCmd::CreateCursorAtPos { view, pos })),
        }
    } else {
        cmds.push(FCmd::Action(ActionCmd::MoveCursorToPos {
            view,
            pos,
            move_anchor: vse.mode() != ViewStoreTypes::Mode::VisualLine,
        }));
    }

    cmds
}

pub fn paste(s: &State, view: ViewId, text: &str) -> Vec<FCmd> {
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();
    if text.is_empty() || dse.read_only {
        return Vec::new();
    }

    let doc = s.index.view_to_doc(view).unwrap();

    let mut cmds = vec![
        FCmd::Action(ActionCmd::EndCommit { doc }),
        FCmd::Action(ActionCmd::StartCommit { doc }),
        FCmd::Action(ActionCmd::Insert { view, text: text.to_string() }),
        FCmd::Action(ActionCmd::EndCommit { doc }),
    ];

    if vse.mode() == ViewStoreTypes::Mode::Insert {
        cmds.push(FCmd::Action(ActionCmd::StartCommit { doc }));
    }

    cmds
}

pub fn quit(s: &State, view: ViewId) -> Option<Vec<FCmd>> {
    let window = s.index.view_to_window(view)?;
    let fallback = s.workspace.tiles().into_iter().find(|&w| w != window);

    fallback.map_or_else(
        || Some(vec![FCmd::Quit]),
        |fallback| {
            Some(vec![
                FCmd::DocView(DocViewCmd::DestroyView { view }),
                FCmd::Action(ActionCmd::SetActiveWindow { window: Some(fallback) }),
            ])
        },
    )
}

pub fn scroll_view(s: &State, view: ViewId, motion: Motion) -> Vec<FCmd> {
    let vse = s.view_store.get(&view).unwrap();

    let mut scroll = *vse.scroll;

    match motion {
        Motion::Up => scroll.y = scroll.y.saturating_sub(1),
        Motion::Down => scroll.y = scroll.y.saturating_add(1),
        Motion::Left => scroll.x = scroll.x.saturating_sub(1),
        Motion::Right => scroll.x = scroll.x.saturating_add(1),
        _ => {}
    }

    vec![FCmd::View(ViewCmd::ScrollTo { view, pos: scroll })]
}

pub fn jump(s: &State, view: ViewId, move_anchor: bool) -> Option<Vec<FCmd>> {
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();
    let cursor = vse.cursors.list.first()?;

    let pos = util::offset_to_pos(cursor.offset, vse, dse);

    let mut x = pos.x;
    let mut y = pos.y;

    let pcallback = Box::new(move |s: &mut State, res: Option<String>| {
        let Some(res) = res else { return };
        if res.is_empty() {
            return;
        }

        let parts: Vec<&str> = res.split(':').collect();

        if !parts.is_empty()
            && !parts[0].is_empty()
            && let Ok(val) = parts[0].parse::<usize>()
        {
            y = val.saturating_sub(1);
        }
        if parts.len() > 1
            && !parts[1].is_empty()
            && let Ok(val) = parts[1].parse::<usize>()
        {
            x = val.saturating_sub(1);
        }

        ActionProtocol::move_cursor_to_pos(s, view, Pos::new(x, y), move_anchor);
    });

    Some(vec![FCmd::Mp(MpCmd::Prompt {
        prompt: "Jump to: ".to_string(),
        initial_text: None,
        pcallback,
        tx: None,
    })])
}

pub fn focus_window(s: &State, motion: Motion) -> Option<Vec<FCmd>> {
    s.workspace
        .navigate(motion)
        .map(|window| vec![FCmd::Action(ActionCmd::SetActiveWindow { window: Some(window) })])
}

pub fn yank(s: &State, view: ViewId, motion: Motion) -> Option<Vec<FCmd>> {
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let offsets = util::motion::offsets(motion, vse, dse);
    if offsets.is_empty() {
        return None;
    }

    let mut yanked = String::new();
    for &(start, end) in &offsets {
        for chunk in dse.doc.data.chunks(start.min(end)..start.max(end)) {
            yanked.push_str(chunk);
        }

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

pub fn yank_line(s: &State, view: ViewId) -> Option<Vec<FCmd>> {
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let mut lines: Vec<_> =
        vse.cursors.list.iter().map(|c| dse.doc.data.line_idx_at_offset(c.offset)).collect();
    lines.sort_unstable();
    lines.dedup();

    if lines.is_empty() {
        return None;
    }

    let mut yanked = String::new();
    for &y in &lines {
        let start = dse.doc.data.line_start_offset(y);
        let mut end = dse.doc.data.line_end_offset(y);

        // Remove the newline if not the last line.
        if y + 1 != dse.doc.data.line_count() {
            end -= 1;
        }

        for chunk in dse.doc.data.chunks(start..end) {
            yanked.push_str(chunk);
        }

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

pub fn indentation(
    s: &State, view: ViewId, mut lines: Vec<usize>, dedent: bool,
) -> Option<Vec<FCmd>> {
    let doc = s.index.view_to_doc(view).unwrap();
    let dse = s.doc_store.get(&doc).unwrap();

    if dse.read_only {
        return None;
    }

    lines.sort_unstable();
    lines.dedup();

    let mut cmds = Vec::new();
    if dedent {
        // FIXME: option to also respect leading spaces.
        for y in lines.into_iter().rev() {
            let start = dse.doc.data.line_start_offset(y);
            let end = dse.doc.data.line_end_offset(y);

            if dse.doc.data.chunks(start..end).next().is_some_and(|chunk| chunk.starts_with('\t')) {
                cmds.push(FCmd::Action(ActionCmd::Remove { view, offset: start, len: 1 }));
            }
        }
    } else {
        // FIXME: option to convert tab to spaces.
        for y in lines.into_iter().rev() {
            cmds.push(FCmd::Action(ActionCmd::InsertAt {
                view,
                text: "\t".to_string(),
                offset: dse.doc.data.line_start_offset(y),
            }));
        }
    }

    if cmds.is_empty() {
        return None;
    }

    cmds.insert(0, FCmd::Action(ActionCmd::StartCommit { doc }));
    cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));

    Some(cmds)
}

pub fn swap_lines(
    s: &State, view: ViewId, doc: DocId, mut lines: Vec<usize>, down: bool,
) -> Option<Vec<FCmd>> {
    lines.sort_unstable();
    lines.dedup();
    if lines.is_empty() {
        return None;
    }

    let (_, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let doc_lines = dse.doc.data.line_count();

    // Group contiguous lines into blocks and move them together.
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    for &y in &lines {
        if let Some(last) = blocks.last_mut()
            && last.1 + 1 == y
        {
            last.1 = y;
            continue;
        }

        blocks.push((y, y));
    }

    let mut cmds = Vec::new();
    if down {
        for &(start, end) in blocks.iter().rev() {
            if end + 1 >= doc_lines {
                continue;
            }

            let block_start = dse.doc.data.line_start_offset(start);
            let block_end = dse.doc.data.line_end_offset(end);

            let target = end + 1;
            let target_start = dse.doc.data.line_start_offset(target);
            let target_end = dse.doc.data.line_end_offset(target);
            let target_text = dse.doc.data.slice(target_start..target_end);

            // Target line is the last line of the document.
            if target + 1 == doc_lines {
                let mut target_text = target_text;
                target_text.push('\n');

                cmds.push(FCmd::Action(ActionCmd::Remove {
                    view,
                    offset: target_start,
                    len: target_end - target_start,
                }));

                // Remove the newline of not the block since it will be at the
                // end.
                cmds.push(FCmd::Action(ActionCmd::Remove { view, offset: block_end - 1, len: 1 }));
                cmds.push(FCmd::Action(ActionCmd::InsertAt {
                    view,
                    text: target_text,
                    offset: block_start,
                }));
            } else {
                cmds.push(FCmd::Action(ActionCmd::Remove {
                    view,
                    offset: target_start,
                    len: target_end - target_start,
                }));
                cmds.push(FCmd::Action(ActionCmd::InsertAt {
                    view,
                    text: target_text,
                    offset: block_start,
                }));
            }
        }
    } else {
        for &(start, end) in blocks.iter().rev() {
            if start == 0 {
                continue;
            }

            let block_end = dse.doc.data.line_end_offset(end);

            let target = start - 1;
            let target_start = dse.doc.data.line_start_offset(target);
            let target_end = dse.doc.data.line_end_offset(target);
            let target_text = dse.doc.data.slice(target_start..target_end);
            let target_len = target_text.len();

            // Block is the last line(s) of the document.
            if end + 1 == doc_lines {
                // Remove the newline of not block since it wont be
                // at the end.
                let mut target_text = target_text;
                target_text.truncate(target_text.len() - 1);

                cmds.push(FCmd::Action(ActionCmd::Remove {
                    view,
                    offset: target_start,
                    len: target_len,
                }));
                cmds.push(FCmd::Action(ActionCmd::InsertAt {
                    view,
                    text: "\n".to_string(),
                    offset: block_end - target_len,
                }));
                cmds.push(FCmd::Action(ActionCmd::InsertAt {
                    view,
                    text: target_text,
                    offset: block_end - target_len + 1,
                }));
            } else {
                cmds.push(FCmd::Action(ActionCmd::Remove {
                    view,
                    offset: target_start,
                    len: target_len,
                }));
                cmds.push(FCmd::Action(ActionCmd::InsertAt {
                    view,
                    text: target_text,
                    offset: block_end - target_len,
                }));
            }
        }
    }

    if cmds.is_empty() {
        return None;
    }

    cmds.insert(0, FCmd::Action(ActionCmd::StartCommit { doc }));
    cmds.push(FCmd::Action(ActionCmd::EndCommit { doc }));

    Some(cmds)
}

pub fn remove_offsets(view: ViewId, offsets: Vec<(usize, usize)>) -> Vec<FCmd> {
    let mut regions: Vec<_> = offsets.into_iter().map(|(s, e)| (s.min(e), s.max(e))).collect();
    regions.sort_by_key(|&(s, _)| s);

    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in regions {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged.sort_by_key(|&(s, _)| std::cmp::Reverse(s));

    let mut cmds = Vec::new();
    for (start, end) in merged {
        if end > start {
            cmds.push(FCmd::Action(ActionCmd::Remove { view, offset: start, len: end - start }));
        }
    }

    cmds
}

pub fn split_window(s: &State, doc: DocId, direction: RectSplit) -> Option<Vec<FCmd>> {
    let window = s.workspace.active_window?;

    let (tx, rx) = oneshot::channel();
    let async_fcmd_tx = s.async_fcmd_tx.clone();
    tokio::spawn(async move {
        let (_, window) = rx.await.unwrap();

        let _ =
            async_fcmd_tx.send(FCmd::Action(ActionCmd::SetActiveWindow { window: Some(window) }));
    });

    Some(vec![FCmd::DocView(DocViewCmd::CreateTile {
        doc,
        view: None,
        split_window: window,
        direction,
        raw: false,
        tx: Some(tx),
    })])
}
pub fn save_file(s: &State, doc: DocId) -> Vec<FCmd> {
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

    let pcallback = Box::new(move |s: &mut State, res: Option<String>| {
        let Some(res) = res else { return };
        if res.is_empty() {
            return;
        }

        let dse = s.doc_store.get(&doc).unwrap();

        let path = util::path::normalize(res);
        let data = dse.doc.data.slice(0..dse.doc.data.len());

        let cb_path = path.clone();
        let async_fcmd_tx = s.async_fcmd_tx.clone();
        let wcallback = Box::new(move |res| -> IoFuture {
            Box::pin(async move { save_file_completion(async_fcmd_tx, res, doc, cb_path) })
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
            let _ =
                async_fcmd_tx.send(FCmd::Action(ActionCmd::SetDocPath { doc, path: Some(path) }));
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

pub fn find_file(s: &State) -> Vec<FCmd> {
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
            DirProtocol::exec(s, DirCmd::Show { window, path: Some(entry.path) });
        } else {
            DocViewProtocol::exec(
                s,
                DocViewCmd::OpenFile { window, path: entry.path, force: false },
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

pub fn select_theme() -> Vec<FCmd> {
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
