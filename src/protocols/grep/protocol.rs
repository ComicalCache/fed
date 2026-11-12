use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use fancy_regex::Regex;

use crate::decorators::SpanDecorator;
use crate::fed::FCmd;
use crate::piece_table::PieceTable;
use crate::protocols::action::ActionProtocol;
use crate::protocols::doc_view::DocViewProtocol;
use crate::protocols::grep::GrepCmd;
use crate::protocols::grep::GrepRenderer;
use crate::protocols::grep::pstate::GrepDocument;
use crate::protocols::grep::pstate::GrepFile;
use crate::protocols::grep::pstate::GrepScan;
use crate::protocols::io::IoProtocol;
use crate::protocols::mp::MpKind;
use crate::protocols::mp::MpProtocol;
use crate::protocols::view::ViewProtocol;
use crate::state::DocId;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::state::ViewStoreTypes::SearchMatch;
use crate::state::ViewStoreTypes::SearchState;
use crate::types::render::WindowId;
use crate::util;

pub struct GrepProtocol {}

impl GrepProtocol {
    pub fn exec(s: &mut State, cmd: GrepCmd) {
        match cmd {
            GrepCmd::Search { window } => Self::search(s, window),
            GrepCmd::SearchCompletion { window, views, query, result, revision } => {
                Self::search_completion(s, window, &views, query, result, revision);
            }
            GrepCmd::Show { window } => Self::show(s, window),
            GrepCmd::Navigate { forward } => Self::navigate(s, forward),

            GrepCmd::SelectResult => Self::select_result(s),
            GrepCmd::SelectResultCompletion { window, file, position, result, revision } => {
                Self::select_result_completion(s, window, file, position, result, revision);
            }

            GrepCmd::Replace { all } => Self::replace(s, all),
            GrepCmd::ReplaceCompletion { target, replacement, result, revision } => {
                Self::replace_completion(s, target.as_ref(), replacement, result, revision);
            }
        }
    }

    pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
        let view = match event {
            ViewStoreTypes::Event::CursorsMoved { view }
            | ViewStoreTypes::Event::CursorsChanged { view } => *view,
            _ => return,
        };
        if view != s.p.grep.view {
            return;
        }

        let vse = s.view_store.get_mut(&view).unwrap();
        if vse.mode() == ViewStoreTypes::Mode::Search {
            return;
        }

        let cursor = vse.cursors.list.first().unwrap();
        let Some(search) = vse.search.as_mut() else { return };
        let Some(index) =
            s.p.grep.matches.partition_point(|m| m.start <= cursor.offset).checked_sub(1)
        else {
            return;
        };

        let m = &s.p.grep.matches[index];
        if cursor.offset == m.start || cursor.offset < m.end {
            search.last_match = Some(index);
        }
    }

    pub fn search(s: &mut State, window: WindowId) {
        if s.p.mp.kind == MpKind::Prompt || s.workspace.get_rect(window).is_none() {
            return;
        }

        let revision = s.p.grep.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let callback = Box::new(move |s: &mut State, query: Option<String>| {
            let Some(query) = query else { return };

            if query.is_empty() || revision != s.p.grep.revision.load(Ordering::Relaxed) {
                return;
            }

            let Some(views) = s.index.window_to_views(window).cloned() else { return };
            let opened_docs = Self::opened_docs(s);
            let curr_revision = s.p.grep.revision.clone();

            let tx = s.async_fcmd_tx.clone();
            tokio::task::spawn_blocking(move || {
                let result = std::env::current_dir()
                    .map_err(|e| e.to_string())
                    .and_then(|root| {
                        GrepScan::new(root, &query, opened_docs, revision, &curr_revision)
                    })
                    .map(|scan| {
                        let document = GrepDocument::new(&scan.files, &scan.root);
                        (scan, document)
                    });

                let _ = tx.send(FCmd::Grep(GrepCmd::SearchCompletion {
                    window,
                    views,
                    query,
                    result,
                    revision,
                }));
            });
        });

        MpProtocol::prompt(s, "Search pwd: ".to_string(), None, callback, None);
    }

    pub fn search_completion(
        s: &mut State, window: WindowId, views: &[ViewId], query: String,
        result: Result<(GrepScan, GrepDocument), String>, revision: u64,
    ) {
        if revision != s.p.grep.revision.load(Ordering::Relaxed)
            || s.index.window_to_views(window).map(Vec::as_slice) != Some(views)
        {
            return;
        }

        let (scan, document) = match result {
            Ok(result) => result,
            Err(error) => {
                util::mp::message(s, error, Duration::from_secs(3));
                return;
            }
        };

        s.p.grep.root = scan.root;
        s.p.grep.query = query;
        s.p.grep.replacement = None;
        s.p.grep.files = scan.files;

        Self::render(s, document, None);
        Self::show(s, window);

        if let Some(first) = scan.errors.first() {
            util::mp::message(
                s,
                format!("{} search error(s): {first}", scan.errors.len()),
                Duration::from_secs(3),
            );
        }
    }

    pub fn show(s: &mut State, window: WindowId) {
        if s.workspace.get_rect(window).is_none() {
            return;
        }

        if s.p.grep.rows.is_empty() {
            let document = GrepDocument::new(&s.p.grep.files, &s.p.grep.root);
            Self::render(s, document, None);
        }

        s.replace_view(window, s.p.grep.view, Box::new(GrepRenderer::new()));

        ActionProtocol::set_active_window(s, Some(window));
        ViewProtocol::resize(s);
    }

    pub fn navigate(s: &mut State, forward: bool) {
        let matches = &s.p.grep.matches;
        if matches.is_empty() {
            return;
        }

        let offset = s.view_store.get(&s.p.grep.view).unwrap().cursors.list[0].offset;
        let target = if forward {
            let next = matches.partition_point(|m| m.start <= offset);
            matches.get(next).unwrap_or(&matches[0]).start
        } else {
            let next = matches.partition_point(|m| m.start < offset);
            matches[next.checked_sub(1).unwrap_or(matches.len() - 1)].start
        };

        Self::set_cursor(s, s.p.grep.view, target);
    }

    pub fn select_result(s: &mut State) {
        let Some(window) = s.index.view_to_window(s.p.grep.view) else { return };
        let (vse, dse) =
            State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, s.p.grep.view).unwrap();

        let offset = vse.cursors.list[0].offset;
        let line = dse.doc.data.line_idx_at_offset(offset);
        let Some(row) = s.p.grep.rows.get(line).copied().flatten() else { return };

        let position = (row.line, offset - dse.doc.data.line_start_offset(line));
        let path = s.p.grep.files[row.file].path.clone();
        let doc = s
            .doc_store
            .iter()
            .find(|(_, dse)| dse.doc.path.as_ref() == Some(&path))
            .map(|(&doc, _)| doc);

        if let Some(doc) = doc {
            Self::open_result(s, doc, window, position);
            return;
        }

        let revision = s.p.grep.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let tx = s.async_fcmd_tx.clone();
        IoProtocol::read(
            s,
            path,
            Box::new(move |result| {
                Box::pin(async move {
                    let _ = tx.send(FCmd::Grep(GrepCmd::SelectResultCompletion {
                        window,
                        file: row.file,
                        position,
                        result,
                        revision,
                    }));
                })
            }),
        );
    }

    pub fn select_result_completion(
        s: &mut State, window: WindowId, file: usize, position: (usize, usize),
        result: Result<String, String>, revision: u64,
    ) {
        if revision != s.p.grep.revision.load(Ordering::Relaxed)
            || s.index.view_to_window(s.p.grep.view) != Some(window)
        {
            return;
        }

        let text = match result {
            Ok(text) => text,
            Err(error) => {
                util::mp::message(s, error, Duration::from_secs(3));
                return;
            }
        };

        let path = s.p.grep.files[file].path.clone();
        let doc = match s.create_doc(Some(path)) {
            Ok(doc) => {
                s.doc_store.get_mut(&doc).unwrap().doc.data = PieceTable::from(text);
                doc
            }
            Err(doc) => doc,
        };

        Self::open_result(s, doc, window, position);
    }

    pub fn replace(s: &mut State, all: bool) {
        if s.p.mp.kind == MpKind::Prompt || s.p.grep.matches.is_empty() {
            return;
        }

        let target = if all {
            None
        } else {
            let offset = s.view_store.get(&s.p.grep.view).unwrap().cursors.list[0].offset;
            let matches = &s.p.grep.matches;
            let next = matches.partition_point(|m| {
                if m.start == m.end { m.start < offset } else { m.end <= offset }
            });

            let m = matches.get(next).unwrap_or(&matches[0]);
            let file = &s.p.grep.files[m.file];

            Some((file.path.clone(), file.matches[m.r#match].start))
        };

        if let Some(replacement) = s.p.grep.replacement.clone() {
            Self::_replace(s, target, replacement);
            return;
        }

        let revision = s.p.grep.revision.load(Ordering::Relaxed);
        let callback = Box::new(move |s: &mut State, replacement: Option<String>| {
            let Some(replacement) = replacement else { return };

            if revision != s.p.grep.revision.load(Ordering::Relaxed) {
                return;
            }

            Self::_replace(s, target, replacement);
        });

        let prompt = if all { "Replace all with: " } else { "Replace with: " };
        MpProtocol::prompt(s, prompt.to_string(), None, callback, None);
    }

    pub fn replace_completion(
        s: &mut State, target: Option<&(PathBuf, usize)>, replacement: String,
        result: Result<HashMap<PathBuf, String>, String>, revision: u64,
    ) {
        if revision != s.p.grep.revision.load(Ordering::Relaxed) {
            return;
        }

        let texts = match result {
            Ok(texts) => texts,
            Err(error) => {
                util::mp::message(s, error, Duration::from_secs(3));
                return;
            }
        };

        let query = s.p.grep.query.clone();
        let mut files: Vec<_> =
            s.p.grep
                .files
                .iter()
                .filter(|f| target.is_none_or(|(p, _)| &f.path == p))
                .cloned()
                .collect();
        let offset = target.map(|(_, offset)| *offset);
        let (replacement_end, skipped) =
            match Self::apply_replacement(s, &mut files, &texts, &query, &replacement, offset) {
                Ok(result) => result,
                Err(error) => {
                    util::mp::message(s, error, Duration::from_secs(3));
                    return;
                }
            };

        // Update the replaced files in pstate.
        if let Some((path, _)) = target {
            s.p.grep.files.retain(|file| &file.path != path);
            s.p.grep.files.extend(files);
            s.p.grep.files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        } else {
            s.p.grep.files = files;
        }

        s.p.grep.files.retain(|file| !file.matches.is_empty());
        s.p.grep.replacement = Some(replacement);

        let document = GrepDocument::new(&s.p.grep.files, &s.p.grep.root);
        let target =
            target.map(|(path, offset)| (path.as_path(), replacement_end.unwrap_or(*offset)));
        Self::render(s, document, target);

        if skipped > 0 {
            util::mp::message(
                s,
                format!("Skipped {skipped} changed match(es)"),
                Duration::from_secs(3),
            );
        }
    }

    fn opened_docs(s: &State) -> HashMap<PathBuf, String> {
        s.doc_store
            .values()
            .filter(|dse| !dse.mode.singleton())
            .filter_map(|dse| dse.doc.path.as_ref().map(|p| (p.clone(), dse.doc.data.slice(..))))
            .collect()
    }

    fn _replace(s: &State, target: Option<(PathBuf, usize)>, replacement: String) {
        let paths: Vec<_> =
            s.p.grep
                .files
                .iter()
                .filter(|f| target.as_ref().is_none_or(|(p, _)| &f.path == p))
                .filter(|f| !s.doc_store.values().any(|dse| dse.doc.path.as_ref() == Some(&f.path)))
                .map(|f| f.path.clone())
                .collect();
        let revision = s.p.grep.revision.fetch_add(1, Ordering::Relaxed) + 1;

        let tx = s.async_fcmd_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = paths
                .into_iter()
                .map(|path| {
                    std::fs::read_to_string(&path)
                        .map(|text| (path.clone(), text))
                        .map_err(|err| format!("{}: {err}", path.display()))
                })
                .collect();

            let _ = tx.send(FCmd::Grep(GrepCmd::ReplaceCompletion {
                target,
                replacement,
                result,
                revision,
            }));
        });
    }

    fn apply_replacement(
        s: &mut State, files: &mut [GrepFile], texts: &HashMap<PathBuf, String>, query: &str,
        replacement: &str, target: Option<usize>,
    ) -> Result<(Option<usize>, usize), String> {
        let regex = Regex::new(query).map_err(|err| format!("Invalid regex: {err}"))?;
        let mut plans = Vec::new();
        let mut next = None;
        let mut skipped = 0;

        // Batch prepare all the modifications before applying them.
        for file in files.iter() {
            let doc = s
                .doc_store
                .iter()
                .find_map(|(&doc, dse)| (dse.doc.path.as_ref() == Some(&file.path)).then_some(doc));
            let text = match doc {
                Some(doc) => s.doc_store[&doc].doc.data.slice(..),
                None => texts
                    .get(&file.path)
                    .cloned()
                    .ok_or_else(|| format!("{} is no longer open", file.path.display()))?,
            };

            let (matches, skipped_matches) =
                Self::valid_matches(file, &text, &regex, replacement, target)?;
            skipped += skipped_matches;

            let mut edits = Vec::new();
            let mut updated =
                GrepFile { path: file.path.clone(), text: file.text.clone(), matches: Vec::new() };
            let mut removed = 0;
            let mut inserted = 0;

            // If the replacement is still valid save it for editing, otherwise
            // just shift the following matches.
            for mut m in matches {
                if let Some(new_text) = &m.replacement {
                    if target.is_some() {
                        next = Some(m.start - removed + inserted + new_text.len());
                    }

                    removed += m.end - m.start;
                    inserted += new_text.len();

                    edits.push(m);
                } else {
                    m.start = m.start - removed + inserted;
                    m.end = m.end - removed + inserted;

                    updated.matches.push(m);
                }
            }

            for m in edits.iter().rev() {
                updated.text.replace_range(m.start..m.end, m.replacement.as_ref().unwrap());
            }

            edits.retain(|m| text[m.start..m.end] != *m.replacement.as_ref().unwrap());
            if !edits.is_empty()
                && let Some(doc) = doc
                && (s.doc_store[&doc].read_only || s.doc_store[&doc].mode.singleton())
            {
                return Err(format!("{} is read only", file.path.display()));
            }

            plans.push((doc, text, edits, updated));
        }

        // Batch apply the edits.
        for (file, (doc, text, edits, updated)) in files.iter_mut().zip(plans) {
            if edits.is_empty() {
                *file = updated;
                continue;
            }

            let doc = doc.unwrap_or_else(|| {
                let doc = s.create_doc(Some(file.path.clone())).unwrap();
                s.doc_store.get_mut(&doc).unwrap().doc.data = PieceTable::from(text);
                doc
            });
            let view = s
                .index
                .doc_to_views(doc)
                .and_then(|v| v.first().copied())
                .unwrap_or_else(|| s.create_view(doc));

            // Don't hijack an active commit with the replace.
            let active_commit = s.doc_store[&doc].doc.data.active_commit();
            if active_commit {
                ActionProtocol::end_commit(s, doc);
            }

            ActionProtocol::start_commit(s, doc);
            for m in edits.iter().rev() {
                if m.end > m.start {
                    ActionProtocol::remove(s, view, m.start, m.end - m.start);
                }

                let text = m.replacement.as_ref().unwrap();
                if !text.is_empty() {
                    ActionProtocol::insert_at(s, view, text.clone(), m.start);
                }
            }
            ActionProtocol::end_commit(s, doc);

            if active_commit {
                ActionProtocol::start_commit(s, doc);
            }

            *file = updated;
        }

        Ok((next, skipped))
    }

    fn valid_matches(
        file: &GrepFile, text: &str, regex: &Regex, replacement: &str, target: Option<usize>,
    ) -> Result<(Vec<SearchMatch>, usize), String> {
        let mut matches = file.matches.clone();
        let mut skipped = 0;

        for m in &mut matches {
            if target.is_some_and(|offset| m.start != offset) {
                continue;
            }

            if text.get(m.start..m.end) != file.text.get(m.start..m.end) {
                skipped += 1;
                continue;
            }

            let captures = regex
                .captures_from_pos(text, m.start)
                .map_err(|err| format!("{}: replacement failed: {err}", file.path.display()))?;
            let Some(captures) = captures else {
                skipped += 1;
                continue;
            };

            let found = captures.get(0).unwrap();
            if found.start() != m.start || found.end() != m.end {
                skipped += 1;
                continue;
            }

            // Only prepare replacement if the exact match still exists.
            let mut new_text = String::new();
            captures.expand(replacement, &mut new_text);
            m.replacement = Some(new_text);
        }

        Ok((matches, skipped))
    }

    fn render(s: &mut State, document: GrepDocument, target: Option<(&Path, usize)>) {
        s.view_store.get_mut(&s.p.grep.view).unwrap().search = None;

        let dse = s.doc_store.get_mut(&s.p.grep.doc).unwrap();
        let len = dse.doc.data.len();

        // Temporarily set the mode to not read only for the modifications.
        dse.read_only = false;

        if len > 0 {
            ActionProtocol::remove(s, s.p.grep.view, 0, len);
        }
        ActionProtocol::insert_at(s, s.p.grep.view, document.text, 0);
        ActionProtocol::saved(s, s.p.grep.doc);

        // Reset the read only flag after the modifications.
        s.doc_store.get_mut(&s.p.grep.doc).unwrap().read_only = true;

        ActionProtocol::set_doc_decorator(
            s,
            s.p.grep.doc,
            DocStoreTypes::DecorationId::Span,
            Box::new(SpanDecorator::new(document.headings)),
        );

        let revision = s.doc_store.get(&s.p.grep.doc).unwrap().doc.revision;
        let vse = s.view_store.get_mut(&s.p.grep.view).unwrap();

        vse.search = Some(SearchState::new(s.p.grep.query.clone(), document.highlights, revision));

        s.p.grep.rows = document.rows;
        s.p.grep.matches = document.matches;

        let matches = &s.p.grep.matches;
        let next = target.and_then(|(path, offset)| {
            matches.iter().find(|m| {
                let file = &s.p.grep.files[m.file];
                file.path.as_path() > path
                    || (file.path == path && file.matches[m.r#match].start >= offset)
            })
        });
        let offset = next.or_else(|| matches.first()).map_or(0, |m| m.start);

        Self::set_cursor(s, s.p.grep.view, offset);
        ViewProtocol::resize(s);
    }

    fn open_result(s: &mut State, doc: DocId, window: WindowId, (line, column): (usize, usize)) {
        let views = s.index.doc_to_views(doc);
        let visible =
            views.and_then(|v| v.iter().find_map(|&v| s.index.view_to_window(v).map(|w| (v, w))));

        let view = views.and_then(|v| v.first()).copied();
        let (view, window) = visible
            .unwrap_or_else(|| (DocViewProtocol::show(s, doc, view, window, false, None), window));

        let data = &s.doc_store.get(&doc).unwrap().doc.data;
        let start = data.line_start_offset(line);
        let source = data.slice(start..data.line_end_offset(line));
        let mut column = column.min(source.trim_end_matches('\n').len());

        // Move the cursor to a save position in case the file changed since the
        // initial search was issued.
        while !source.is_char_boundary(column) {
            column -= 1;
        }

        Self::set_cursor(s, view, start + column);
        ActionProtocol::set_active_window(s, Some(window));
    }

    fn set_cursor(s: &mut State, view: ViewId, offset: usize) {
        let vse = s.view_store.get(&view).unwrap();
        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();

        ActionProtocol::remove_cursors(s, view, &offsets);
        ActionProtocol::create_cursors_at_offsets(s, view, vec![offset]);
    }
}
