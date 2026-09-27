use piece_table::{PieceTable, Slice};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    render,
    state::{DocStoreEntry, DocStoreTypes, ViewStoreEntry, ViewStoreTypes},
    types::{Cursor, Motion},
};

pub fn motion_offsets(
    motion: Motion, vse: &ViewStoreEntry, dse: &DocStoreEntry,
) -> Vec<(usize, usize)> {
    let mut offsets = Vec::new();
    for cursor in &vse.cursors.list {
        let mut target = *cursor;
        apply_motion(&mut target, motion, &dse.doc.data, vse.tab_width, &vse.decs, &dse.decs);

        offsets.push((cursor.offset, target.offset));
    }

    offsets
}

pub fn apply_motion(
    cursor: &mut Cursor, motion: Motion, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    vse_decs: &ViewStoreTypes::Decorations, dse_decs: &DocStoreTypes::Decorations,
) {
    match motion {
        Motion::Up => up(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::Down => down(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::Left => left(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::Right => right(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::BeginningOfLine => beginning_of_line(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::EndOfLine => end_of_line(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::BeginningOfFile => beginning_of_file(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::EndOfFile => end_of_file(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::NextWord => next_word(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::NextWordEnd => next_word_end(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::PrevWord => prev_word(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::PrevWordEnd => prev_word_end(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::NextWhitespace => next_whitespace(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::PrevWhitespace => prev_whitespace(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::NextEmptyLine => next_empty_line(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::PrevEmptyLine => prev_empty_line(cursor, doc, tab_width, dse_decs, vse_decs),
        Motion::MatchingOpposite => matching_opposite(cursor, doc, tab_width, dse_decs, vse_decs),
    }
}

fn up(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let y = y(doc, cursor.offset);
    if y == 0 {
        cursor.offset = 0;
        cursor.pref_x = 0;
        return;
    }

    let start = doc.get_line_start_byte(y - 1);
    let end = doc.get_line_end_byte(y - 1);

    let mut decs = Vec::new();
    dse_decs.range(start, end, &mut decs);
    vse_decs.range(start, end, &mut decs);

    let (vom, _) = render::layout_vom(&doc.slice(start..end), start, tab_width, &decs);
    let vo = vom
        .iter()
        .rev()
        .find(|vo| vo.visual_x <= cursor.pref_x)
        .unwrap_or_else(|| vom.first().unwrap());

    cursor.offset = vo.offset;
}

fn down(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let y = y(doc, cursor.offset);
    let lines = doc.lines();
    if y + 1 == lines {
        cursor.offset = doc.len();
        update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
        return;
    }

    let start = doc.get_line_start_byte(y + 1);
    let end = doc.get_line_end_byte(y + 1);

    let mut decs = Vec::new();
    dse_decs.range(start, end, &mut decs);
    vse_decs.range(start, end, &mut decs);

    let (vom, _) = render::layout_vom(&doc.slice(start..end), start, tab_width, &decs);
    let vo = vom
        .iter()
        .rev()
        .find(|vo| vo.visual_x <= cursor.pref_x)
        .unwrap_or_else(|| vom.first().unwrap());

    cursor.offset = vo.offset;
}

fn left(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    if step_backward(doc, &mut offset) {
        cursor.offset = offset;
        update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
    }
}

fn right(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    if step_forward(doc, &mut offset) {
        cursor.offset = offset;
        update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
    }
}

fn beginning_of_line(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    cursor.offset = doc.get_line_start_byte(y(doc, cursor.offset));
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn end_of_line(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let y = y(doc, cursor.offset);
    let start = doc.get_line_start_byte(y);
    let end = doc.get_line_end_byte(y);

    // Do not treat the newline character as a "character".
    cursor.offset = if doc.slice(start..end).ends_with('\n') { end.saturating_sub(1) } else { end };

    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn beginning_of_file(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    cursor.offset = 0;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn end_of_file(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    cursor.offset = doc.len();
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn next_word(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    let Some(ch) = char_at(doc, offset) else { return };

    if ch.is_alphanumeric() {
        while let Some(ch) = char_at(doc, offset) {
            if !ch.is_alphanumeric() || !step_forward(doc, &mut offset) {
                break;
            }
        }
        while let Some(ch) = char_at(doc, offset) {
            if !ch.is_whitespace() || !step_forward(doc, &mut offset) {
                break;
            }
        }
    } else if ch.is_whitespace() {
        while let Some(ch) = char_at(doc, offset) {
            if !ch.is_whitespace() || !step_forward(doc, &mut offset) {
                break;
            }
        }
    } else {
        step_forward(doc, &mut offset);
        while let Some(ch) = char_at(doc, offset) {
            if !ch.is_whitespace() || !step_forward(doc, &mut offset) {
                break;
            }
        }
    }

    cursor.offset = offset;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn next_word_end(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    let Some(ch) = char_at(doc, offset) else { return };

    if ch.is_alphanumeric() {
        while let Some(ch) = char_at(doc, offset) {
            if !ch.is_alphanumeric() || !step_forward(doc, &mut offset) {
                break;
            }
        }
    } else if ch.is_whitespace() {
        while let Some(ch) = char_at(doc, offset) {
            if !ch.is_whitespace() || !step_forward(doc, &mut offset) {
                break;
            }
        }
        if let Some(ch) = char_at(doc, offset) {
            if ch.is_alphanumeric() {
                while let Some(ch) = char_at(doc, offset) {
                    if !ch.is_alphanumeric() || !step_forward(doc, &mut offset) {
                        break;
                    }
                }
            } else {
                step_forward(doc, &mut offset);
            }
        }
    } else {
        step_forward(doc, &mut offset);
    }

    cursor.offset = offset;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn prev_word(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    if !step_backward(doc, &mut offset) {
        return;
    }

    let ch = char_at(doc, offset).unwrap();
    if ch.is_alphanumeric() {
        while step_backward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset) {
                if !ch.is_alphanumeric() {
                    step_forward(doc, &mut offset);
                    break;
                }
            }
        }
    } else if ch.is_whitespace() {
        while step_backward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset) {
                if !ch.is_whitespace() {
                    break;
                }
            }
        }

        if let Some(ch) = char_at(doc, offset)
            && ch.is_alphanumeric()
        {
            while step_backward(doc, &mut offset) {
                if let Some(ch) = char_at(doc, offset)
                    && !ch.is_alphanumeric()
                {
                    step_forward(doc, &mut offset);
                    break;
                }
            }
        }
    } else {
        // Punctuation.
    }

    cursor.offset = offset;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn prev_word_end(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    if !step_backward(doc, &mut offset) {
        return;
    }

    let ch = char_at(doc, offset).unwrap();
    if ch.is_alphanumeric() {
        while step_backward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset)
                && !ch.is_alphanumeric()
            {
                break;
            }
        }
        while step_backward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset)
                && !ch.is_whitespace()
            {
                step_forward(doc, &mut offset);
                break;
            }
        }
    } else if ch.is_whitespace() {
        while step_backward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset)
                && !ch.is_whitespace()
            {
                step_forward(doc, &mut offset);
                break;
            }
        }
    } else {
        // Punctuation.
    }

    cursor.offset = offset;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn next_whitespace(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;

    while let Some(ch) = char_at(doc, offset) {
        if !ch.is_whitespace() {
            break;
        }
        if !step_forward(doc, &mut offset) {
            break;
        }
    }

    while let Some(ch) = char_at(doc, offset) {
        if ch.is_whitespace() {
            break;
        }
        if !step_forward(doc, &mut offset) {
            break;
        }
    }

    cursor.offset = offset;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn prev_whitespace(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let mut offset = cursor.offset;
    if !step_backward(doc, &mut offset) {
        return;
    }

    while let Some(ch) = char_at(doc, offset) {
        if !ch.is_whitespace() {
            break;
        }
        if !step_backward(doc, &mut offset) {
            break;
        }
    }

    while let Some(ch) = char_at(doc, offset) {
        if ch.is_whitespace() {
            break;
        }
        if !step_backward(doc, &mut offset) {
            break;
        }
    }

    while let Some(ch) = char_at(doc, offset) {
        if !ch.is_whitespace() {
            break;
        }
        if !step_backward(doc, &mut offset) {
            break;
        }
    }

    step_forward(doc, &mut offset);

    cursor.offset = offset;
    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn next_empty_line(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let y = y(doc, cursor.offset);
    let lines = doc.lines();

    let mut found = false;
    for target in (y + 1)..lines {
        let start = doc.get_line_start_byte(target);
        let end = doc.get_line_end_byte(target);
        let line = doc.slice(start..end);

        if line.is_empty() || line == "\n" {
            cursor.offset = start;
            found = true;

            break;
        }
    }

    if !found {
        cursor.offset = doc.len();
    }

    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn prev_empty_line(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let y = y(doc, cursor.offset);

    if y == 0 {
        update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
        return;
    }

    let mut found = false;
    for target in (0..y).rev() {
        let start = doc.get_line_start_byte(target);
        let end = doc.get_line_end_byte(target);
        let line = doc.slice(start..end);

        if line.is_empty() || line == "\n" {
            cursor.offset = start;
            found = true;

            break;
        }
    }

    if !found {
        cursor.offset = 0;
    }

    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn matching_opposite(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let Some(start_char) = char_at(doc, cursor.offset) else { return };

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
        while step_forward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset) {
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
        while step_backward(doc, &mut offset) {
            if let Some(ch) = char_at(doc, offset) {
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

    update_pref_x(cursor, doc, tab_width, dse_decs, vse_decs);
}

fn char_at(doc: &PieceTable, offset: usize) -> Option<char> {
    if offset >= doc.len() {
        return None;
    }

    let y = y(doc, offset);
    let end = doc.get_line_end_byte(y);

    let text = doc.slice(offset..end);
    text.graphemes(true).next().and_then(|g| g.chars().next())
}

fn step_forward(doc: &PieceTable, offset: &mut usize) -> bool {
    if *offset >= doc.len() {
        return false;
    }

    let y = y(doc, *offset);
    let end = doc.get_line_end_byte(y);

    let text = doc.slice(*offset..end);
    if let Some(g) = text.graphemes(true).next() {
        *offset += g.len();
        true
    } else {
        false
    }
}

fn step_backward(doc: &PieceTable, offset: &mut usize) -> bool {
    if *offset == 0 {
        return false;
    }

    let y = y(doc, *offset - 1);
    let start = doc.get_line_start_byte(y);

    let text = doc.slice(start..*offset);
    if let Some(g) = text.graphemes(true).next_back() {
        *offset -= g.len();
        true
    } else {
        false
    }
}

fn y(doc: &PieceTable, offset: usize) -> usize {
    let lines = doc.lines();

    (0..lines)
        .find(|&y| {
            offset >= doc.get_line_start_byte(y)
                && (offset < doc.get_line_end_byte(y) || y == lines - 1)
        })
        .unwrap_or(lines.saturating_sub(1))
}

fn update_pref_x(
    cursor: &mut Cursor, doc: &PieceTable, tab_width: ViewStoreTypes::TabWidth,
    dse_decs: &DocStoreTypes::Decorations, vse_decs: &ViewStoreTypes::Decorations,
) {
    let y = y(doc, cursor.offset);
    let start = doc.get_line_start_byte(y);
    let end = doc.get_line_end_byte(y);
    let line = doc.slice(start..end);

    let mut decs = Vec::new();
    dse_decs.range(start, end, &mut decs);
    vse_decs.range(start, end, &mut decs);

    let (vom, _) = render::layout_vom(&line, start, tab_width, &decs);
    cursor.pref_x =
        vom.iter().find(|vo| vo.offset == cursor.offset).map(|vo| vo.visual_x).unwrap_or(0);
}
