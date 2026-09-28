use piece_table::Slice;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    render,
    state::{DocStoreEntry, ViewStoreEntry},
    types::{Cursor, Motion},
};

pub fn motion_offsets(
    motion: Motion, vse: &ViewStoreEntry, dse: &DocStoreEntry,
) -> Vec<(usize, usize)> {
    let mut offsets = Vec::new();
    for cursor in &vse.cursors.list {
        let mut target = *cursor;
        apply_motion(&mut target, motion, vse, dse);

        offsets.push((cursor.offset, target.offset));
    }

    offsets
}

pub fn apply_motion(
    cursor: &mut Cursor, motion: Motion, vse: &ViewStoreEntry, dse: &DocStoreEntry,
) {
    match motion {
        Motion::Up => up(cursor, vse, dse),
        Motion::Down => down(cursor, vse, dse),
        Motion::Left => left(cursor, vse, dse),
        Motion::Right => right(cursor, vse, dse),
        Motion::BeginningOfLine => beginning_of_line(cursor, vse, dse),
        Motion::EndOfLine => end_of_line(cursor, vse, dse),
        Motion::BeginningOfFile => beginning_of_file(cursor, vse, dse),
        Motion::EndOfFile => end_of_file(cursor, vse, dse),
        Motion::NextWord => next_word(cursor, vse, dse),
        Motion::NextWordEnd => next_word_end(cursor, vse, dse),
        Motion::PrevWord => prev_word(cursor, vse, dse),
        Motion::PrevWordEnd => prev_word_end(cursor, vse, dse),
        Motion::NextWhitespace => next_whitespace(cursor, vse, dse),
        Motion::PrevWhitespace => prev_whitespace(cursor, vse, dse),
        Motion::NextEmptyLine => next_empty_line(cursor, vse, dse),
        Motion::PrevEmptyLine => prev_empty_line(cursor, vse, dse),
        Motion::MatchingOpposite => matching_opposite(cursor, vse, dse),
    }
}

fn up(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = y(cursor.offset, dse);
    if y == 0 {
        cursor.offset = 0;
        cursor.pref_x = 0;
        return;
    }

    let start = dse.doc.data.get_line_start_byte(y - 1);
    let end = dse.doc.data.get_line_end_byte(y - 1);

    let mut decs = Vec::new();
    dse.decs.range(start, end, &mut decs);
    vse.decs.range(start, end, &mut decs);

    let (vom, _) = render::layout_vom(
        &dse.doc.data.slice(start..end),
        start,
        &decs,
        &vse.layout.replacements,
        vse.tab_width,
    );
    let vo = vom
        .iter()
        .rev()
        .find(|vo| vo.visual_x <= cursor.pref_x)
        .unwrap_or_else(|| vom.first().unwrap());

    cursor.offset = vo.offset;
}

fn down(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = y(cursor.offset, dse);
    let lines = dse.doc.data.lines();
    if y + 1 == lines {
        cursor.offset = dse.doc.data.len();
        update_pref_x(cursor, vse, dse);
        return;
    }

    let start = dse.doc.data.get_line_start_byte(y + 1);
    let end = dse.doc.data.get_line_end_byte(y + 1);

    let mut decs = Vec::new();
    dse.decs.range(start, end, &mut decs);
    vse.decs.range(start, end, &mut decs);

    let (vom, _) = render::layout_vom(
        &dse.doc.data.slice(start..end),
        start,
        &decs,
        &vse.layout.replacements,
        vse.tab_width,
    );
    let vo = vom
        .iter()
        .rev()
        .find(|vo| vo.visual_x <= cursor.pref_x)
        .unwrap_or_else(|| vom.first().unwrap());

    cursor.offset = vo.offset;
}

fn left(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    if step_backward(&mut offset, dse) {
        cursor.offset = offset;
        update_pref_x(cursor, vse, dse);
    }
}

fn right(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    if step_forward(&mut offset, dse) {
        cursor.offset = offset;
        update_pref_x(cursor, vse, dse);
    }
}

fn beginning_of_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    cursor.offset = dse.doc.data.get_line_start_byte(y(cursor.offset, dse));
    update_pref_x(cursor, vse, dse);
}

fn end_of_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = y(cursor.offset, dse);
    let start = dse.doc.data.get_line_start_byte(y);
    let end = dse.doc.data.get_line_end_byte(y);

    // Do not treat the newline character as a "character".
    cursor.offset =
        if dse.doc.data.slice(start..end).ends_with('\n') { end.saturating_sub(1) } else { end };

    update_pref_x(cursor, vse, dse);
}

fn beginning_of_file(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    cursor.offset = 0;
    update_pref_x(cursor, vse, dse);
}

fn end_of_file(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    cursor.offset = dse.doc.data.len();
    update_pref_x(cursor, vse, dse);
}

fn next_word(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    let Some(ch) = char_at(offset, dse) else { return };

    if ch.is_alphanumeric() {
        while let Some(ch) = char_at(offset, dse) {
            if !ch.is_alphanumeric() || !step_forward(&mut offset, dse) {
                break;
            }
        }
        while let Some(ch) = char_at(offset, dse) {
            if !ch.is_whitespace() || !step_forward(&mut offset, dse) {
                break;
            }
        }
    } else if ch.is_whitespace() {
        while let Some(ch) = char_at(offset, dse) {
            if !ch.is_whitespace() || !step_forward(&mut offset, dse) {
                break;
            }
        }
    } else {
        step_forward(&mut offset, dse);
        while let Some(ch) = char_at(offset, dse) {
            if !ch.is_whitespace() || !step_forward(&mut offset, dse) {
                break;
            }
        }
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn next_word_end(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    let Some(ch) = char_at(offset, dse) else { return };

    if ch.is_alphanumeric() {
        while let Some(ch) = char_at(offset, dse) {
            if !ch.is_alphanumeric() || !step_forward(&mut offset, dse) {
                break;
            }
        }
    } else if ch.is_whitespace() {
        while let Some(ch) = char_at(offset, dse) {
            if !ch.is_whitespace() || !step_forward(&mut offset, dse) {
                break;
            }
        }
        if let Some(ch) = char_at(offset, dse) {
            if ch.is_alphanumeric() {
                while let Some(ch) = char_at(offset, dse) {
                    if !ch.is_alphanumeric() || !step_forward(&mut offset, dse) {
                        break;
                    }
                }
            } else {
                step_forward(&mut offset, dse);
            }
        }
    } else {
        step_forward(&mut offset, dse);
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn prev_word(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    if !step_backward(&mut offset, dse) {
        return;
    }

    let ch = char_at(offset, dse).unwrap();
    if ch.is_alphanumeric() {
        while step_backward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse) {
                if !ch.is_alphanumeric() {
                    step_forward(&mut offset, dse);
                    break;
                }
            }
        }
    } else if ch.is_whitespace() {
        while step_backward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse) {
                if !ch.is_whitespace() {
                    break;
                }
            }
        }

        if let Some(ch) = char_at(offset, dse)
            && ch.is_alphanumeric()
        {
            while step_backward(&mut offset, dse) {
                if let Some(ch) = char_at(offset, dse)
                    && !ch.is_alphanumeric()
                {
                    step_forward(&mut offset, dse);
                    break;
                }
            }
        }
    } else {
        // Punctuation.
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn prev_word_end(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    if !step_backward(&mut offset, dse) {
        return;
    }

    let ch = char_at(offset, dse).unwrap();
    if ch.is_alphanumeric() {
        while step_backward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse)
                && !ch.is_alphanumeric()
            {
                break;
            }
        }
        while step_backward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse)
                && !ch.is_whitespace()
            {
                step_forward(&mut offset, dse);
                break;
            }
        }
    } else if ch.is_whitespace() {
        while step_backward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse)
                && !ch.is_whitespace()
            {
                step_forward(&mut offset, dse);
                break;
            }
        }
    } else {
        // Punctuation.
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn next_whitespace(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;

    while let Some(ch) = char_at(offset, dse) {
        if !ch.is_whitespace() {
            break;
        }
        if !step_forward(&mut offset, dse) {
            break;
        }
    }

    while let Some(ch) = char_at(offset, dse) {
        if ch.is_whitespace() {
            break;
        }
        if !step_forward(&mut offset, dse) {
            break;
        }
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn prev_whitespace(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut offset = cursor.offset;
    if !step_backward(&mut offset, dse) {
        return;
    }

    while let Some(ch) = char_at(offset, dse) {
        if !ch.is_whitespace() {
            break;
        }
        if !step_backward(&mut offset, dse) {
            break;
        }
    }

    while let Some(ch) = char_at(offset, dse) {
        if ch.is_whitespace() {
            break;
        }
        if !step_backward(&mut offset, dse) {
            break;
        }
    }

    while let Some(ch) = char_at(offset, dse) {
        if !ch.is_whitespace() {
            break;
        }
        if !step_backward(&mut offset, dse) {
            break;
        }
    }

    step_forward(&mut offset, dse);

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn next_empty_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = y(cursor.offset, dse);
    let lines = dse.doc.data.lines();

    let mut found = false;
    for target in (y + 1)..lines {
        let start = dse.doc.data.get_line_start_byte(target);
        let end = dse.doc.data.get_line_end_byte(target);
        let line = dse.doc.data.slice(start..end);

        if line.is_empty() || line == "\n" {
            cursor.offset = start;
            found = true;

            break;
        }
    }

    if !found {
        cursor.offset = dse.doc.data.len();
    }

    update_pref_x(cursor, vse, dse);
}

fn prev_empty_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = y(cursor.offset, dse);

    if y == 0 {
        update_pref_x(cursor, vse, dse);
        return;
    }

    let mut found = false;
    for target in (0..y).rev() {
        let start = dse.doc.data.get_line_start_byte(target);
        let end = dse.doc.data.get_line_end_byte(target);
        let line = dse.doc.data.slice(start..end);

        if line.is_empty() || line == "\n" {
            cursor.offset = start;
            found = true;

            break;
        }
    }

    if !found {
        cursor.offset = 0;
    }

    update_pref_x(cursor, vse, dse);
}

fn matching_opposite(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let Some(start_char) = char_at(cursor.offset, dse) else { return };

    let (opening, closing, forward) = match start_char {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        '{' => ('{', '}', true),
        '<' => ('<', '>', true),
        ')' => (')', '(', false),
        ']' => (']', '[', false),
        '}' => ('}', '{', false),
        '>' => ('>', '<', false),
        _ => return,
    };

    let reset = cursor.offset;

    let mut offset = cursor.offset;
    let mut depth = 1;
    if forward {
        while step_forward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse) {
                if ch == opening {
                    depth += 1;
                } else if ch == closing {
                    depth -= 1;
                }

                if depth == 0 {
                    cursor.offset = offset;
                    break;
                }
            }
        }
    } else {
        while step_backward(&mut offset, dse) {
            if let Some(ch) = char_at(offset, dse) {
                if ch == opening {
                    depth += 1;
                } else if ch == closing {
                    depth -= 1;
                }

                if depth == 0 {
                    cursor.offset = offset;
                    break;
                }
            }
        }
    }

    if depth != 0 {
        cursor.offset = reset;
    }

    update_pref_x(cursor, vse, dse);
}

fn char_at(offset: usize, dse: &DocStoreEntry) -> Option<char> {
    if offset >= dse.doc.data.len() {
        return None;
    }

    let y = y(offset, dse);
    let end = dse.doc.data.get_line_end_byte(y);

    let text = dse.doc.data.slice(offset..end);
    text.graphemes(true).next().and_then(|g| g.chars().next())
}

fn step_forward(offset: &mut usize, dse: &DocStoreEntry) -> bool {
    if *offset >= dse.doc.data.len() {
        return false;
    }

    let y = y(*offset, dse);
    let end = dse.doc.data.get_line_end_byte(y);

    let text = dse.doc.data.slice(*offset..end);
    if let Some(g) = text.graphemes(true).next() {
        *offset += g.len();
        true
    } else {
        false
    }
}

fn step_backward(offset: &mut usize, dse: &DocStoreEntry) -> bool {
    if *offset == 0 {
        return false;
    }

    let y = y(*offset - 1, dse);
    let start = dse.doc.data.get_line_start_byte(y);

    let text = dse.doc.data.slice(start..*offset);
    if let Some(g) = text.graphemes(true).next_back() {
        *offset -= g.len();
        true
    } else {
        false
    }
}

fn y(offset: usize, dse: &DocStoreEntry) -> usize {
    let lines = dse.doc.data.lines();

    (0..lines)
        .find(|&y| {
            offset >= dse.doc.data.get_line_start_byte(y)
                && (offset < dse.doc.data.get_line_end_byte(y) || y == lines - 1)
        })
        .unwrap_or(lines.saturating_sub(1))
}

fn update_pref_x(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = y(cursor.offset, dse);
    let start = dse.doc.data.get_line_start_byte(y);
    let end = dse.doc.data.get_line_end_byte(y);
    let line = dse.doc.data.slice(start..end);

    let mut decs = Vec::new();
    dse.decs.range(start, end, &mut decs);
    vse.decs.range(start, end, &mut decs);

    let (vom, _) = render::layout_vom(&line, start, &decs, &vse.layout.replacements, vse.tab_width);
    cursor.pref_x =
        vom.iter().find(|vo| vo.offset == cursor.offset).map(|vo| vo.visual_x).unwrap_or(0);
}
