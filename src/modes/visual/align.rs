use std::time::Duration;

use fancy_regex::Regex;

use crate::fed::FCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::mp::MpCmd;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::util;

pub fn prompt(s: &State, view: ViewId, mut lines: Vec<usize>) -> Vec<FCmd> {
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

        let edits = match edits(s, view, &lines, &pattern) {
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

        if edits.is_empty() {
            return;
        }

        ActionProtocol::start_commit(s, doc);
        for (offset, len, text) in edits.into_iter().rev() {
            if len > 0 {
                ActionProtocol::remove(s, view, offset, len);
            }

            ActionProtocol::insert_at(s, view, text, offset);
        }
        ActionProtocol::end_commit(s, doc);
    });

    vec![FCmd::Mp(MpCmd::Prompt {
        prompt: "Align at: ".to_string(),
        initial_text: None,
        pcallback,
        tx: None,
    })]
}

fn edits(
    s: &State, view: ViewId, lines: &[usize], pattern: &str,
) -> Result<Vec<(usize, usize, String)>, fancy_regex::Error> {
    // Computing the regex synchronously should be fine in this case since it is
    // usually only on few selected lines and not the entire document.
    let regex = Regex::new(pattern)?;
    let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

    let mut edits = Vec::new();
    for block in lines.chunk_by(|a, b| *a + 1 == *b) {
        let mut matches = Vec::new();
        for &y in block {
            let start = dse.doc.data.line_start_offset(y);
            let mut end = dse.doc.data.line_end_offset(y);
            if y + 1 != dse.doc.data.line_count() {
                end -= 1;
            }

            let text = dse.doc.data.slice(start..end);
            let Some(found) = regex.find(&text)? else { continue };
            let prefix = text[..found.start()].trim_end_matches([' ', '\t']);

            // If the prefix is only whitespace, don't align it.
            if prefix.trim().is_empty() {
                continue;
            }

            let offset = start + prefix.len();
            let len = found.start() - prefix.len();
            let column = util::offset_to_pos(offset, vse, dse).x;
            matches.push((offset, len, column));
        }

        let Some(target) = matches.iter().map(|&(_, _, col)| col + 1).max() else {
            continue;
        };

        for (offset, len, column) in matches {
            let text = " ".repeat(target - column);
            if text != dse.doc.data.slice(offset..offset + len) {
                edits.push((offset, len, text));
            }
        }
    }

    Ok(edits)
}
