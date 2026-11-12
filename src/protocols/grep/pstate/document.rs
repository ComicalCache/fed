use std::path::Path;

use crate::protocols::grep::pstate::GrepFile;
use crate::protocols::grep::pstate::GrepMatch;
use crate::protocols::grep::pstate::GrepRow;
use crate::state::ViewStoreTypes::SearchMatch;
use crate::types::Decoration;
use crate::types::Face;
use crate::types::Span;

pub struct GrepDocument {
    pub text: String,

    pub rows: Vec<Option<GrepRow>>,
    pub matches: Vec<GrepMatch>,
    pub highlights: Vec<SearchMatch>,
    pub headings: Vec<Span<Decoration>>,
}

impl GrepDocument {
    pub fn new(files: &[GrepFile], root: &Path) -> Self {
        let mut doc = GrepDocument {
            text: String::new(),
            rows: Vec::new(),
            matches: Vec::new(),
            highlights: Vec::new(),
            headings: Vec::new(),
        };

        for (file_idx, file) in files.iter().enumerate() {
            if file.matches.is_empty() {
                continue;
            }

            // Add spacer between file entries.
            if !doc.rows.is_empty() {
                doc.push(None, "");
            }

            let relative_path = file.path.strip_prefix(root).unwrap_or(&file.path);
            let heading =
                format!("{}:", relative_path.display()).replace('\n', "\\n").replace('\r', "\\r");

            // Add heading.
            let start = doc.push(None, &heading);
            doc.headings.push(Span {
                start,
                end: start + heading.len(),
                data: Decoration::Style { face: Face { bold: Some(true), ..Face::default() } },
            });

            let mut line_start_bytes = vec![0];
            line_start_bytes.extend(file.text.match_indices('\n').map(|(offset, _)| offset + 1));

            let line_range = |m: &SearchMatch| {
                let first =
                    line_start_bytes.partition_point(|&start| start <= m.start).saturating_sub(1);

                let last = if m.start == m.end {
                    first
                } else {
                    line_start_bytes.partition_point(|&start| start < m.end).saturating_sub(1)
                };

                (first, last)
            };
            let line_end_byte = |line: usize| {
                line_start_bytes.get(line + 1).map_or(file.text.len(), |&next| next - 1)
            };

            // Init mapping between file lines and generated document offsets.
            let mut grep_doc_offsets = vec![None; line_start_bytes.len()];
            for m in &file.matches {
                let (first, last) = line_range(m);
                grep_doc_offsets[first..=last].fill(Some(0));
            }

            // Compute mappings between file lines and generated document
            // offsets.
            for (line, offset) in grep_doc_offsets.iter_mut().enumerate() {
                if offset.is_none() {
                    continue;
                }

                *offset = Some(doc.push(
                    Some(GrepRow { file: file_idx, line }),
                    &file.text[line_start_bytes[line]..line_end_byte(line)],
                ));
            }

            for (idx, m) in file.matches.iter().enumerate() {
                let (first, last) = line_range(m);
                let mut span = GrepMatch { file: file_idx, r#match: idx, start: 0, end: 0 };

                // Add search highlights to all matches.
                for line in first..=last {
                    let offset = grep_doc_offsets[line].unwrap();
                    let end = line_end_byte(line);

                    let start = offset + m.start.clamp(line_start_bytes[line], end)
                        - line_start_bytes[line];
                    let end =
                        offset + m.end.clamp(line_start_bytes[line], end) - line_start_bytes[line];

                    doc.highlights.push(SearchMatch { start, end, replacement: None });

                    if line == first {
                        span.start = start;
                    }

                    span.end = end;
                }

                doc.matches.push(span);
            }
        }

        if doc.rows.is_empty() {
            doc.push(None, "No matches");
        }

        doc.highlights.sort_unstable_by_key(|m| (m.start, m.end));
        doc
    }

    fn push(&mut self, row: Option<GrepRow>, text: &str) -> usize {
        if !self.rows.is_empty() {
            self.text.push('\n');
        }

        let start = self.text.len();

        self.text.push_str(text);
        self.rows.push(row);

        start
    }
}
