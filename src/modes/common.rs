use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use piece_table::Slice;
use tokio::sync::oneshot;

use crate::{
    fed::FCmd,
    protocols::{
        action::{ActionCmd, ActionProtocol},
        doc_view::DocViewCmd,
        mp::MpCmd,
        view::ViewCmd,
    },
    state::{DocId, State, ViewId},
    types::{Motion, Pos, RectSplit, render::WindowId},
    util,
};

pub fn click(s: &State, window: WindowId, view: ViewId, event: MouseEvent) -> Vec<FCmd> {
    if event.kind != MouseEventKind::Down(MouseButton::Left) {
        return Vec::new();
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
        cmds.push(FCmd::Action(ActionCmd::MoveCursorToPos { view, pos, move_anchor: true }));
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

pub fn jump(s: &State, view: ViewId) -> Option<Vec<FCmd>> {
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();
    let cursor = vse.cursors.list.first()?;

    let pos = util::offset_to_pos(cursor.offset, vse, dse);

    let mut x = pos.x;
    let mut y = pos.y;

    let pcallback = Box::new(move |s: &mut State, res: String| {
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

        ActionProtocol::move_cursor_to_pos(s, view, Pos::new(x, y), true);
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
        yanked.push_str(&dse.doc.data.slice(start.min(end)..start.max(end)));

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

        // Remove the newline if not the last line.
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

pub fn swap_lines(
    s: &State, view: ViewId, doc: DocId, mut lines: Vec<usize>, down: bool,
) -> Option<Vec<FCmd>> {
    lines.sort_unstable();
    lines.dedup();
    if lines.is_empty() {
        return None;
    }

    let (_, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let doc_lines = dse.doc.data.lines();

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

            let block_start = dse.doc.data.get_line_start_byte(start);
            let block_end = dse.doc.data.get_line_end_byte(end);

            let target = end + 1;
            let target_start = dse.doc.data.get_line_start_byte(target);
            let target_end = dse.doc.data.get_line_end_byte(target);
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

                // Remove the newline of not the block since it will be
                // at the end. This will break for Windows CRLF line
                // endings.
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

            let block_end = dse.doc.data.get_line_end_byte(end);

            let target = start - 1;
            let target_start = dse.doc.data.get_line_start_byte(target);
            let target_end = dse.doc.data.get_line_end_byte(target);
            let target_text = dse.doc.data.slice(target_start..target_end);
            let target_len = target_text.len();

            // Block is the last line(s) of the document.
            if end + 1 == doc_lines {
                // Remove the newline of not block since it wont be
                // at the end. This will break for Windows CRLF line
                // endings.
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
