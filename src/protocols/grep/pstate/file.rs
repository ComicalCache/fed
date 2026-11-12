use std::path::PathBuf;

use fancy_regex::Regex;

use crate::state::DocId;
use crate::state::DocStore;
use crate::state::ViewStoreTypes::SearchMatch;

#[derive(Clone)]
pub struct GrepFile {
    pub path: PathBuf,
    pub doc: Option<DocId>,
    pub text: String,

    pub matches: Vec<SearchMatch>,
}

impl GrepFile {
    pub fn new(path: PathBuf, text: String, regex: &Regex) -> Result<Self, String> {
        let matches = regex
            .find_iter(&text)
            .map(|result| {
                result.map(|m| SearchMatch { start: m.start(), end: m.end(), replacement: None })
            })
            .collect::<Result<_, _>>()
            .map_err(|err| format!("{}: {err}", path.display()))?;
        Ok(GrepFile { path, doc: None, text, matches })
    }

    pub fn doc(&self, docs: &DocStore) -> Option<DocId> {
        self.doc.filter(|doc| docs.contains_key(doc)).or_else(|| {
            docs.iter()
                .find_map(|(&doc, dse)| (dse.doc.path.as_ref() == Some(&self.path)).then_some(doc))
        })
    }

    pub fn has_pending_matches(&self, target: Option<usize>) -> bool {
        self.matches
            .iter()
            .any(|m| m.replacement.is_none() && target.is_none_or(|offset| m.start == offset))
    }

    pub fn replacements(
        &self, text: &str, regex: &Regex, replacement: &str, target: Option<usize>,
    ) -> Result<(Vec<(usize, SearchMatch)>, usize), String> {
        let mut matches = Vec::new();
        let mut skipped = 0;
        let mut removed = 0;
        let mut inserted = 0;

        for (idx, m) in self.matches.iter().enumerate() {
            if let Some(replacement) = &m.replacement {
                removed += m.end - m.start;
                inserted += replacement.len();
                continue;
            }

            if target.is_some_and(|offset| m.start != offset) {
                continue;
            }

            let start = m.start - removed + inserted;
            let end = m.end - removed + inserted;
            if text.get(start..end) != self.text.get(m.start..m.end) {
                skipped += 1;
                continue;
            }

            let captures = regex
                .captures_from_pos(text, start)
                .map_err(|err| format!("{}: replacement failed: {err}", self.path.display()))?;
            let Some(captures) = captures else {
                skipped += 1;
                continue;
            };

            let found = captures.get(0).unwrap();
            if found.start() != start || found.end() != end {
                skipped += 1;
                continue;
            }

            let mut new_text = String::new();
            captures.expand(replacement, &mut new_text);

            matches.push((idx, SearchMatch { start, end, replacement: Some(new_text) }));
        }

        Ok((matches, skipped))
    }

    pub fn replacement_offset(&self, offset: usize) -> usize {
        let mut removed = 0;
        let mut inserted = 0;
        for m in &self.matches {
            if m.start > offset {
                break;
            }

            if let Some(replacement) = &m.replacement {
                if offset < m.end {
                    return m.start - removed
                        + inserted
                        + (offset - m.start).min(replacement.len());
                }

                removed += m.end - m.start;
                inserted += replacement.len();
            }
        }

        offset - removed + inserted
    }
}
