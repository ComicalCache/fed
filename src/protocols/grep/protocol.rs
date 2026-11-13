use std::collections::HashMap;
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
            GrepCmd::UndoAll => Self::undo_all(s),
            GrepCmd::HotRedoAll => Self::hot_redo_all(s),
            GrepCmd::SaveAll => Self::save_all(s),
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
            let undos: HashMap<_, _> = s
                .doc_store
                .iter()
                .filter(|(_, dse)| !dse.mode.singleton())
                .filter_map(|(&doc, dse)| {
                    let path = dse.doc.path.as_ref()?;
                    Some((path.clone(), (doc, dse.doc.data.commit_id())))
                })
                .collect();
            let curr_revision = s.p.grep.revision.clone();

            let tx = s.async_fcmd_tx.clone();
            tokio::task::spawn_blocking(move || {
                let result = std::env::current_dir()
                    .map_err(|e| e.to_string())
                    .and_then(|root| {
                        GrepScan::new(root, &query, opened_docs, revision, &curr_revision)
                    })
                    .map(|mut scan| {
                        for file in &mut scan.files {
                            if let Some(&(doc, undo)) = undos.get(&file.path) {
                                file.doc = Some(doc);
                                file.undo = undo;
                            }
                        }
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

        let (mut scan, document) = match result {
            Ok(result) => result,
            Err(error) => {
                util::mp::message(s, error, Duration::from_secs(3));
                return;
            }
        };

        // Link files containing matches with already opened docs.
        for file in &mut scan.files {
            file.doc = file.doc(&s.doc_store);
        }

        s.p.grep.root = scan.root;
        s.p.grep.query = query;
        s.p.grep.replacement = None;
        s.p.grep.files = scan.files;

        Self::render(s, document, true);
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
            Self::render(s, document, true);
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
            s.p.grep.next_pending_match(next)
        } else {
            let next = matches.partition_point(|m| m.start < offset);
            matches[..next]
                .iter()
                .rev()
                .chain(matches[next..].iter().rev())
                .find(|m| s.p.grep.files[m.file].matches[m.r#match].replacement.is_none())
        };

        if let Some(target) = target {
            Self::set_cursor(s, s.p.grep.view, target.start);
        }
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
        let doc = s.p.grep.files[row.file].doc(&s.doc_store);

        if let Some(doc) = doc {
            Self::open_result(s, doc, window, row.file, position);
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

        Self::open_result(s, doc, window, file, position);
    }

    pub fn replace(s: &mut State, all: bool) {
        if s.p.mp.kind == MpKind::Prompt || s.p.grep.next_pending_match(0).is_none() {
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

            let Some(m) = s.p.grep.next_pending_match(next) else { return };
            let file = &s.p.grep.files[m.file];

            Some((file.path.clone(), file.matches[m.r#match].start))
        };

        if let Some(replacement) = s.p.grep.replacement.clone() {
            Self::prepare_replacement(s, target, replacement);
            return;
        }

        let revision = s.p.grep.revision.load(Ordering::Relaxed);
        let callback = Box::new(move |s: &mut State, replacement: Option<String>| {
            let Some(replacement) = replacement else { return };

            if revision != s.p.grep.revision.load(Ordering::Relaxed) {
                return;
            }

            Self::prepare_replacement(s, target, replacement);
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

        let skipped = match Self::apply_replacement(s, &texts, &replacement, target) {
            Ok(skipped) => skipped,
            Err(error) => {
                util::mp::message(s, error, Duration::from_secs(3));
                return;
            }
        };
        s.p.grep.replacement = Some(replacement);

        let document = GrepDocument::new(&s.p.grep.files, &s.p.grep.root);
        Self::render(s, document, false);

        // Move the cursor to the next result after the replaced one, if one
        // exists.
        if let Some((path, offset)) = target {
            let matches = &s.p.grep.matches;
            let idx = matches.iter().position(|m| {
                let file = &s.p.grep.files[m.file];
                &file.path == path && file.matches[m.r#match].start == *offset
            });

            if let Some(idx) = idx
                && s.p.grep.files[matches[idx].file].matches[matches[idx].r#match]
                    .replacement
                    .is_some()
            {
                let next = s.p.grep.next_pending_match(idx + 1).unwrap_or(&matches[idx]).start;
                Self::set_cursor(s, s.p.grep.view, next);
            }
        }

        if skipped > 0 {
            util::mp::message(
                s,
                format!("Skipped {skipped} changed match(es)"),
                Duration::from_secs(3),
            );
        }
    }

    pub fn undo_all(s: &mut State) {
        s.p.grep.revision.fetch_add(1, Ordering::Relaxed);

        for file_idx in 0..s.p.grep.files.len() {
            let file = &s.p.grep.files[file_idx];
            if let Some(doc) = file.doc(&s.doc_store) {
                let dse = &s.doc_store[&doc];
                let undo = file.undo;

                if dse.read_only || dse.mode.singleton() || !dse.doc.data.can_undo_to(undo) {
                    continue;
                }

                let active_commit = dse.doc.data.active_commit();
                if active_commit {
                    ActionProtocol::end_commit(s, doc);

                    let commit = s.doc_store[&doc].doc.data.commit_id();
                    if commit != undo {
                        s.p.grep.files[file_idx].record_replacement(commit);
                    }
                }

                let view = s
                    .index
                    .doc_to_views(doc)
                    .and_then(|v| v.first().copied())
                    .unwrap_or_else(|| s.create_view(doc));
                while s.doc_store[&doc].doc.data.commit_id() != undo {
                    ActionProtocol::undo(s, view);
                }

                if active_commit {
                    ActionProtocol::start_commit(s, doc);
                }
            } else if file.doc.is_some() {
                // Document has been closed since searching.
                continue;
            }

            // Reset replacements for proper display and to be able to replace
            // again.
            for m in &mut s.p.grep.files[file_idx].matches {
                m.replacement = None;
            }
            s.p.grep.replacement = None;
        }

        let document = GrepDocument::new(&s.p.grep.files, &s.p.grep.root);
        Self::render(s, document, false);
    }

    pub fn hot_redo_all(s: &mut State) {
        s.p.grep.revision.fetch_add(1, Ordering::Relaxed);

        for file_idx in 0..s.p.grep.files.len() {
            let file = &s.p.grep.files[file_idx];
            let Some((redo, _)) = &file.redo else { continue };
            let redo = *redo;

            if let Some(doc) = file.doc(&s.doc_store) {
                let dse = &s.doc_store[&doc];

                if dse.read_only || dse.mode.singleton() || !dse.doc.data.can_hot_redo_to(redo) {
                    continue;
                }

                let view = s
                    .index
                    .doc_to_views(doc)
                    .and_then(|v| v.first().copied())
                    .unwrap_or_else(|| s.create_view(doc));
                while s.doc_store[&doc].doc.data.commit_id() != redo {
                    ActionProtocol::hot_redo(s, view);
                }
            } else if file.doc.is_some() {
                // Document has been closed.
                continue;
            }

            // Restore replacements for proper display and to avoid "replacing"
            // already replaced matches.
            let file = &mut s.p.grep.files[file_idx];
            let (_, replacements) = file.redo.as_ref().unwrap();
            for (m, replacement) in file.matches.iter_mut().zip(replacements) {
                m.replacement.clone_from(replacement);
            }
        }

        let document = GrepDocument::new(&s.p.grep.files, &s.p.grep.root);
        Self::render(s, document, false);
    }

    pub fn save_all(s: &mut State) {
        let docs: HashMap<_, _> =
            s.p.grep
                .files
                .iter()
                .filter_map(|file| {
                    let doc = file.doc(&s.doc_store)?;
                    let dse = s.doc_store.get(&doc).unwrap();

                    if !dse.doc.modified || dse.read_only || dse.mode.singleton() {
                        return None;
                    }

                    Some((doc, dse.doc.path.clone()?))
                })
                .collect();

        if docs.is_empty() {
            return;
        }

        for (doc, path) in docs {
            ActionProtocol::save(s, doc, path);
        }
    }

    fn opened_docs(s: &State) -> HashMap<PathBuf, String> {
        s.doc_store
            .values()
            .filter(|dse| !dse.mode.singleton())
            .filter_map(|dse| dse.doc.path.as_ref().map(|p| (p.clone(), dse.doc.data.slice(..))))
            .collect()
    }

    fn prepare_replacement(s: &State, target: Option<(PathBuf, usize)>, replacement: String) {
        let paths: Vec<_> =
            s.p.grep
                .files
                .iter()
                .filter(|f| target.as_ref().is_none_or(|(p, _)| &f.path == p))
                .filter(|f| f.has_pending_matches(target.as_ref().map(|(_, offset)| *offset)))
                .filter(|f| f.doc(&s.doc_store).is_none())
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
        s: &mut State, texts: &HashMap<PathBuf, String>, replacement: &str,
        target: Option<&(PathBuf, usize)>,
    ) -> Result<usize, String> {
        let regex = Regex::new(&s.p.grep.query).map_err(|err| format!("Invalid regex: {err}"))?;
        let offset = target.map(|(_, offset)| *offset);
        let mut plans = Vec::new();
        let mut skipped = 0;

        // Batch prepare all the modifications before applying them.
        for (file_idx, file) in s.p.grep.files.iter().enumerate() {
            if target.is_some_and(|(path, _)| &file.path != path)
                || !file.has_pending_matches(offset)
            {
                continue;
            }

            let doc = file.doc(&s.doc_store);
            let text = match doc {
                Some(doc) => s.doc_store[&doc].doc.data.slice(..),
                None => texts
                    .get(&file.path)
                    .cloned()
                    .ok_or_else(|| format!("{} is no longer open", file.path.display()))?,
            };

            let (matches, skipped_matches) =
                file.replacements(&text, &regex, replacement, offset)?;
            skipped += skipped_matches;

            let edits: Vec<_> = matches
                .iter()
                .filter(|(_, m)| text[m.start..m.end] != *m.replacement.as_ref().unwrap())
                .map(|(_, m)| m.clone())
                .collect();
            if !edits.is_empty()
                && let Some(doc) = doc
                && (s.doc_store[&doc].read_only || s.doc_store[&doc].mode.singleton())
            {
                return Err(format!("{} is read only", file.path.display()));
            }

            plans.push((file_idx, doc, text, edits, matches));
        }

        // Batch apply the edits.
        for (file_idx, doc, text, edits, matches) in plans {
            if matches.is_empty() {
                continue;
            }

            let file = &mut s.p.grep.files[file_idx];
            file.doc = doc;

            for (index, m) in matches {
                file.matches[index].replacement = m.replacement;
            }

            if edits.is_empty() {
                file.record_replacement(
                    doc.map_or(0, |doc| s.doc_store[&doc].doc.data.commit_id()),
                );
                continue;
            }

            let path = file.path.clone();

            let doc = doc.unwrap_or_else(|| {
                let doc = s.create_doc(Some(path)).unwrap();
                s.doc_store.get_mut(&doc).unwrap().doc.data = PieceTable::from(text);
                doc
            });
            let view = s
                .index
                .doc_to_views(doc)
                .and_then(|v| v.first().copied())
                .unwrap_or_else(|| s.create_view(doc));

            s.p.grep.files[file_idx].doc = Some(doc);

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
            s.p.grep.files[file_idx].record_replacement(s.doc_store[&doc].doc.data.commit_id());

            if active_commit {
                ActionProtocol::start_commit(s, doc);
            }
        }

        Ok(skipped)
    }

    fn render(s: &mut State, document: GrepDocument, reset_cursor: bool) {
        let offset = s.view_store.get(&s.p.grep.view).unwrap().cursors.list[0].offset;
        let anchor = if reset_cursor {
            None
        } else {
            s.p.grep.matches.iter().rev().find(|m| m.start <= offset).copied()
        };
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

        let offset = anchor
            .and_then(|anchor| {
                s.p.grep
                    .matches
                    .iter()
                    .find(|m| m.file == anchor.file && m.r#match == anchor.r#match)
            })
            .or_else(|| s.p.grep.matches.first())
            .map_or(0, |m| m.start);

        Self::set_cursor(s, s.p.grep.view, offset);
        ViewProtocol::resize(s);
    }

    fn open_result(
        s: &mut State, doc: DocId, window: WindowId, file: usize, (line, column): (usize, usize),
    ) {
        s.p.grep.files[file].doc = Some(doc);

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
