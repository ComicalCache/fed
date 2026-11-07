use std::time::Duration;

use fancy_regex::Regex;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::fed::FCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::mp::MpCmd;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::util;

pub fn prompt(s: &State, view: ViewId, mut lines: Vec<usize>, repeated: bool) -> Vec<FCmd> {
    let doc = s.index.view_to_doc(view).unwrap();
    let revision = s.doc_store.get(&doc).unwrap().doc.revision;

    lines.sort_unstable();
    lines.dedup();

    let pcallback = Box::new(move |s: &mut State, pattern: Option<String>| {
        let Some(pattern) = pattern else { return };

        if pattern.is_empty() || s.index.view_to_doc(view) != Some(doc) {
            return;
        }

        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();
        if dse.read_only
            || !matches!(
                vse.mode(),
                ViewStoreTypes::Mode::Visual | ViewStoreTypes::Mode::VisualLine
            )
        {
            return;
        }

        if dse.doc.revision != revision {
            util::mp::message(s, "Document changed".to_string(), Duration::from_secs(3));
            return;
        }

        let edits = match edits(s, view, &lines, &pattern, repeated) {
            Ok(edits) => edits,
            Err(err) => {
                util::mp::message(
                    s,
                    format!("Error aligning lines: {err}"),
                    Duration::from_secs(3),
                );
                return;
            }
        };

        if !edits.is_empty() {
            ActionProtocol::start_commit(s, doc);
            for (offset, len, text) in edits.into_iter().rev() {
                if len > 0 {
                    ActionProtocol::remove(s, view, offset, len);
                }

                ActionProtocol::insert_at(s, view, text, offset);
            }
            ActionProtocol::end_commit(s, doc);
        }

        ActionProtocol::reset_cursor_anchors(s, view);
        ActionProtocol::pop_view_mode(s, view);
    });

    vec![FCmd::Mp(MpCmd::Prompt {
        prompt: if repeated { "Align (repeat): " } else { "Align: " }.to_string(),
        initial_text: None,
        pcallback,
        tx: None,
    })]
}

struct Row {
    start: usize,
    text: String,

    matches: Vec<(usize, usize)>,
    prefix: String,

    consumed: usize,
}

fn edits(
    s: &State, view: ViewId, lines: &[usize], pattern: &str, repeated: bool,
) -> Result<Vec<(usize, usize, String)>, fancy_regex::Error> {
    // Computing the regex synchronously should be fine in this case since it is
    // usually only on few selected lines and not the entire document.
    let regex = Regex::new(pattern)?;
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let limit = if repeated { usize::MAX } else { 1 };

    let mut edits = Vec::new();
    for block in lines.chunk_by(|a, b| *a + 1 == *b) {
        let mut rows = Vec::new();
        for &y in block {
            let start = dse.doc.data.line_start_offset(y);
            let mut end = dse.doc.data.line_end_offset(y);
            if y + 1 != dse.doc.data.line_count() {
                end -= 1;
            }

            let text = dse.doc.data.slice(start..end);
            let matches: Result<_, _> = regex
                .find_iter(&text)
                .take(limit)
                .map(|m| m.map(|m| (m.start(), m.end())))
                .collect();
            let matches = matches?;

            rows.push(Row { start, text, matches, prefix: String::new(), consumed: 0 });
        }

        edits.extend(block_edits(&mut rows, vse.layout.tab_width));
    }

    edits.sort_unstable_by_key(|&(offset, _, _)| offset);
    Ok(edits)
}

fn block_edits(
    rows: &mut [Row], tab_width: ViewStoreTypes::TabWidth,
) -> Vec<(usize, usize, String)> {
    let mut edits = Vec::new();

    let cols = rows.iter().map(|row| row.matches.len()).max().unwrap_or(0);
    for col in 0..cols {
        let mut matches = Vec::new();
        for row in rows.iter_mut() {
            let Some(&(start, end)) = row.matches.get(col) else { continue };

            // Don't replace the previous matches.
            let padding = row.text[..start].trim_end_matches([' ', '\t']).len().max(row.consumed);
            row.prefix.push_str(&row.text[row.consumed..padding]);
            row.consumed = end;

            // If the prefix is only whitespace, don't align it.
            if row.prefix.trim().is_empty() {
                row.prefix.push_str(&row.text[padding..end]);

                continue;
            }

            let width = row.prefix.graphemes(true).fold(0, |x, ch| {
                x + if ch == "\t" { *tab_width - x % *tab_width } else { ch.width() }
            });

            matches.push((row, padding..start, width));
        }

        let Some(target) = matches.iter().map(|(_, _, width)| width + 1).max() else {
            continue;
        };

        for (row, padding, width) in matches {
            let text = " ".repeat(target - width);
            row.prefix.push_str(&text);
            row.prefix.push_str(&row.text[padding.end..row.consumed]);

            if text != row.text[padding.clone()] {
                edits.push((row.start + padding.start, padding.len(), text));
            }
        }
    }

    edits
}
