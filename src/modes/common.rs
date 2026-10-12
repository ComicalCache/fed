use piece_table::Slice;

use crate::{
    debug_panic::debug_panic,
    fed::FCmd,
    protocols::{
        action::{ActionCmd, ActionProtocol},
        doc_view::DocViewCmd,
        mp::MpCmd,
        pstate::PState,
        view::ViewCmd,
    },
    state::{DocId, State, ViewId},
    types::{Motion, Pos},
    util,
};

pub fn quit(state: &State, view: ViewId) -> Option<Vec<FCmd>> {
    let fallback =
        state.workspace.tiles().into_iter().find(|&w| Some(w) != state.workspace.active_window);

    if let Some(fallback) = fallback {
        Some(vec![
            FCmd::DocView(DocViewCmd::DestroyView { view }),
            FCmd::Action(ActionCmd::SetActiveWindow { window: Some(fallback) }),
        ])
    } else {
        Some(vec![FCmd::Quit])
    }
}

pub fn scroll_view(state: &State, view: ViewId, motion: Motion) -> Option<Vec<FCmd>> {
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

pub fn jump(state: &State, view: ViewId) -> Option<Vec<FCmd>> {
    let Some((vse, dse)) =
        State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
    else {
        debug_panic!();
        return None;
    };
    let Some(cursor) = vse.cursors.list.first() else {
        return None;
    };

    let pos = util::offset_to_pos(cursor.offset, vse, dse);

    let mut x = pos.x;
    let mut y = pos.y;

    let pcallback = Box::new(move |state: &mut State, _: &mut PState, res: String| {
        if res.is_empty() {
            return;
        }

        let parts: Vec<&str> = res.split(':').collect();

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

        ActionProtocol::move_cursor_to_pos(state, view, Pos::new(x, y), true);
    });

    Some(vec![FCmd::Mp(MpCmd::Prompt {
        prompt: "Jump to: ".to_string(),
        initial_text: None,
        pcallback,
        tx: None,
    })])
}

pub fn focus_window(state: &State, motion: Motion) -> Option<Vec<FCmd>> {
    if let Some(window) = state.workspace.navigate(motion) {
        Some(vec![FCmd::Action(ActionCmd::SetActiveWindow { window: Some(window) })])
    } else {
        None
    }
}

pub fn yank(state: &State, view: ViewId, motion: Motion) -> Option<Vec<FCmd>> {
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
        yanked.push_str(&dse.doc.data.slice(start.min(end)..start.max(end)));

        // Newline to separate multi-cursor yanks.
        yanked.push('\n');
    }
    yanked.pop();

    if !yanked.is_empty() {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(yanked);
        }
    }

    Some(Vec::new())
}

pub fn yank_line(state: &State, view: ViewId) -> Option<Vec<FCmd>> {
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

        // Remove the newline if not the last line.
        if y + 1 != dse.doc.data.lines() {
            end -= 1;
        }

        yanked.push_str(&dse.doc.data.slice(start..end));

        // Newline to separate multi-cursor yanks.
        yanked.push('\n');
    }
    yanked.pop();

    if !yanked.is_empty() {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(yanked);
        }
    }

    Some(Vec::new())
}

pub fn swap_lines(
    state: &State, view: ViewId, doc: DocId, mut lines: Vec<usize>, down: bool,
) -> Option<Vec<FCmd>> {
    lines.sort_unstable();
    lines.dedup();
    if lines.is_empty() {
        return None;
    }

    let Some((_vse, dse)) =
        State::vse_and_dse(&state.view_store, &state.doc_store, &state.index, view)
    else {
        debug_panic!();
        return None;
    };

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
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
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
