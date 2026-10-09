use piece_table::Slice;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    state::{DocStoreEntry, ViewStoreEntry},
    types::{Cursor, Motion},
    util,
};

pub fn offsets(motion: Motion, vse: &ViewStoreEntry, dse: &DocStoreEntry) -> Vec<(usize, usize)> {
    let mut offsets = Vec::new();
    for cursor in &vse.cursors.list {
        let mut target = *cursor;
        apply(&mut target, motion, vse, dse);

        offsets.push((cursor.offset, target.offset));
    }

    offsets
}

pub fn apply(cursor: &mut Cursor, motion: Motion, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
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
    let y = dse.doc.data.get_line_of_byte(cursor.offset);
    if y == 0 {
        cursor.offset = 0;
        cursor.pref_x = 0;
        return;
    }

    let (vom, _) = util::vom(y - 1, vse, dse);
    let vo = vom
        .iter()
        .rev()
        .find(|vo| vo.visual_x <= cursor.pref_x)
        // FIXME: is this ever none?
        .unwrap_or_else(|| vom.first().unwrap());

    cursor.offset = vo.offset;
}

fn down(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = dse.doc.data.get_line_of_byte(cursor.offset);
    let lines = dse.doc.data.lines();
    if y + 1 >= lines {
        cursor.offset = dse.doc.data.len();
        update_pref_x(cursor, vse, dse);

        return;
    }

    let (vom, _) = util::vom(y + 1, vse, dse);
    let vo = vom
        .iter()
        .rev()
        .find(|vo| vo.visual_x <= cursor.pref_x)
        // FIXME: is this ever none?
        .unwrap_or_else(|| vom.first().unwrap());

    cursor.offset = vo.offset;
}

fn left(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    if reader.step_backward(&mut offset) {
        cursor.offset = offset;
        update_pref_x(cursor, vse, dse);
    }
}

fn right(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    if reader.step_forward(&mut offset) {
        cursor.offset = offset;
        update_pref_x(cursor, vse, dse);
    }
}

fn beginning_of_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    cursor.offset = dse.doc.data.get_line_start_byte(dse.doc.data.get_line_of_byte(cursor.offset));
    update_pref_x(cursor, vse, dse);
}

fn end_of_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = dse.doc.data.get_line_of_byte(cursor.offset);
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
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    let Some(ch) = reader.char_at(offset) else { return };

    if ch.is_alphanumeric() {
        while let Some(ch) = reader.char_at(offset) {
            if !ch.is_alphanumeric() || !reader.step_forward(&mut offset) {
                break;
            }
        }
        while let Some(ch) = reader.char_at(offset) {
            if !ch.is_whitespace() || !reader.step_forward(&mut offset) {
                break;
            }
        }
    } else if ch.is_whitespace() {
        while let Some(ch) = reader.char_at(offset) {
            if !ch.is_whitespace() || !reader.step_forward(&mut offset) {
                break;
            }
        }
    } else {
        reader.step_forward(&mut offset);
        while let Some(ch) = reader.char_at(offset) {
            if !ch.is_whitespace() || !reader.step_forward(&mut offset) {
                break;
            }
        }
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn next_word_end(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    let Some(ch) = reader.char_at(offset) else { return };

    if ch.is_alphanumeric() {
        while let Some(ch) = reader.char_at(offset) {
            if !ch.is_alphanumeric() || !reader.step_forward(&mut offset) {
                break;
            }
        }
    } else if ch.is_whitespace() {
        while let Some(ch) = reader.char_at(offset) {
            if !ch.is_whitespace() || !reader.step_forward(&mut offset) {
                break;
            }
        }

        if let Some(ch) = reader.char_at(offset) {
            if ch.is_alphanumeric() {
                while let Some(ch) = reader.char_at(offset) {
                    if !ch.is_alphanumeric() || !reader.step_forward(&mut offset) {
                        break;
                    }
                }
            } else {
                reader.step_forward(&mut offset);
            }
        }
    } else {
        reader.step_forward(&mut offset);
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn prev_word(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    if !reader.step_backward(&mut offset) {
        return;
    }

    let ch = reader.char_at(offset).unwrap();
    if ch.is_alphanumeric() {
        while reader.step_backward(&mut offset) {
            if let Some(ch) = reader.char_at(offset)
                && !ch.is_alphanumeric()
            {
                reader.step_forward(&mut offset);
                break;
            }
        }
    } else if ch.is_whitespace() {
        while reader.step_backward(&mut offset) {
            if let Some(ch) = reader.char_at(offset)
                && !ch.is_whitespace()
            {
                break;
            }
        }
        if let Some(ch) = reader.char_at(offset) {
            if ch.is_alphanumeric() {
                while reader.step_backward(&mut offset) {
                    if let Some(ch) = reader.char_at(offset)
                        && !ch.is_alphanumeric()
                    {
                        reader.step_forward(&mut offset);
                        break;
                    }
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
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    if !reader.step_backward(&mut offset) {
        return;
    }

    let ch = reader.char_at(offset).unwrap();
    if ch.is_alphanumeric() {
        while reader.step_backward(&mut offset) {
            if let Some(ch) = reader.char_at(offset)
                && !ch.is_alphanumeric()
            {
                break;
            }
        }
        while reader.step_backward(&mut offset) {
            if let Some(ch) = reader.char_at(offset)
                && !ch.is_whitespace()
            {
                reader.step_forward(&mut offset);
                break;
            }
        }
    } else if ch.is_whitespace() {
        while reader.step_backward(&mut offset) {
            if let Some(ch) = reader.char_at(offset)
                && !ch.is_whitespace()
            {
                reader.step_forward(&mut offset);
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
    let mut reader = TextReader::new(dse);
    let mut offset = cursor.offset;

    while let Some(ch) = reader.char_at(offset) {
        if !ch.is_whitespace() || !reader.step_forward(&mut offset) {
            break;
        }
    }

    while let Some(ch) = reader.char_at(offset) {
        if ch.is_whitespace() || !reader.step_forward(&mut offset) {
            break;
        }
    }

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn prev_whitespace(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let mut reader = TextReader::new(dse);

    let mut offset = cursor.offset;
    if !reader.step_backward(&mut offset) {
        return;
    }

    while let Some(ch) = reader.char_at(offset) {
        if !ch.is_whitespace() || !reader.step_backward(&mut offset) {
            break;
        }
    }

    while let Some(ch) = reader.char_at(offset) {
        if ch.is_whitespace() || !reader.step_backward(&mut offset) {
            break;
        }
    }

    while let Some(ch) = reader.char_at(offset) {
        if !ch.is_whitespace() || !reader.step_backward(&mut offset) {
            break;
        }
    }

    reader.step_forward(&mut offset);

    cursor.offset = offset;
    update_pref_x(cursor, vse, dse);
}

fn next_empty_line(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = dse.doc.data.get_line_of_byte(cursor.offset);
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
    let y = dse.doc.data.get_line_of_byte(cursor.offset);

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
    let mut reader = TextReader::new(dse);

    let Some(start_char) = reader.char_at(cursor.offset) else { return };
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
        while reader.step_forward(&mut offset) {
            if let Some(ch) = reader.char_at(offset) {
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
        while reader.step_backward(&mut offset) {
            if let Some(ch) = reader.char_at(offset) {
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

fn update_pref_x(cursor: &mut Cursor, vse: &ViewStoreEntry, dse: &DocStoreEntry) {
    let y = dse.doc.data.get_line_of_byte(cursor.offset);

    let (vom, _) = util::vom(y, vse, dse);
    cursor.pref_x =
        vom.iter().find(|vo| vo.offset == cursor.offset).map(|vo| vo.visual_x).unwrap_or(0);
}

struct TextReader<'a> {
    dse: &'a DocStoreEntry,

    y: usize,
    start: usize,
    end: usize,

    text: String,
}

impl<'a> TextReader<'a> {
    fn new(dse: &'a DocStoreEntry) -> Self {
        Self { dse, y: usize::MAX, start: 0, end: 0, text: String::new() }
    }

    fn fetch(&mut self, offset: usize) {
        let lines = self.dse.doc.data.lines();
        let valid = self.y != usize::MAX
            && offset >= self.start
            && (offset < self.end || (self.y == lines.saturating_sub(1) && offset == self.end));

        if valid {
            return;
        }

        self.y = self.dse.doc.data.get_line_of_byte(offset);
        self.start = self.dse.doc.data.get_line_start_byte(self.y);
        self.end = self.dse.doc.data.get_line_end_byte(self.y);
        self.text = self.dse.doc.data.slice(self.start..self.end);
    }

    fn char_at(&mut self, offset: usize) -> Option<char> {
        if offset >= self.dse.doc.data.len() {
            return None;
        }

        self.fetch(offset);

        self.text[offset - self.start..].graphemes(true).next().and_then(|g| g.chars().next())
    }

    fn step_forward(&mut self, offset: &mut usize) -> bool {
        if *offset >= self.dse.doc.data.len() {
            return false;
        }

        self.fetch(*offset);

        if let Some(grapheme) = self.text[*offset - self.start..].graphemes(true).next() {
            *offset += grapheme.len();

            true
        } else {
            false
        }
    }

    fn step_backward(&mut self, offset: &mut usize) -> bool {
        if *offset == 0 {
            return false;
        }

        self.fetch(*offset - 1);

        if let Some(grapheme) = self.text[..*offset - self.start].graphemes(true).next_back() {
            *offset -= grapheme.len();

            true
        } else {
            false
        }
    }
}
