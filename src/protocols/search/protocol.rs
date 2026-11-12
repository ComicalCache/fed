use std::time::Duration;

use fancy_regex::Regex;

use crate::fed::FCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::mp::MpKind;
use crate::protocols::mp::MpProtocol;
use crate::protocols::search::SearchCmd;
use crate::protocols::search::SearchRequest;
use crate::state::DocId;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::state::ViewStoreTypes::SearchMatch;
use crate::state::ViewStoreTypes::SearchState;
use crate::util;

pub struct SearchProtocol {}

impl SearchProtocol {
    pub fn exec(s: &mut State, cmd: SearchCmd) {
        match cmd {
            SearchCmd::Search { view, doc, bounds } => Self::search(s, view, doc, bounds),
            SearchCmd::SearchCompletion { request, query, matches } => {
                Self::search_completion(s, request, query, matches);
            }
            SearchCmd::Navigate { view, forward } => Self::navigate(s, view, forward),
            SearchCmd::Replace { view, doc, all } => Self::replace(s, view, doc, all),
            SearchCmd::ReplaceCompletion { request, matches, replacement, index } => {
                Self::replace_completion(s, request, matches, replacement, index);
            }
            SearchCmd::Close { view } => Self::close(s, view),

            SearchCmd::Failed { request, message } => {
                if Self::finalize(s, request) {
                    util::mp::message(s, message, Duration::from_secs(3));
                }
            }
        }
    }

    pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
        let doc = match event {
            DocStoreTypes::Event::Inserted { doc, .. }
            | DocStoreTypes::Event::Removed { doc, .. } => *doc,
            _ => return,
        };

        // Event may be received after the doc has already been removed again.
        let Some(dse) = s.doc_store.get(&doc) else { return };

        let revision = dse.doc.revision;
        for view in s.index.doc_to_views(doc).cloned().unwrap_or_default() {
            let stale = s
                .view_store
                .get(&view)
                .and_then(|vse| vse.search.as_ref())
                .is_some_and(|search| search.revision != revision);

            if stale {
                Self::close(s, view);
            } else if s.p.search.searches.get(&view).is_some_and(|r| r.revision != revision) {
                s.p.search.searches.remove(&view);
            }
        }
    }

    pub fn exec_view_event(s: &mut State, event: &ViewStoreTypes::Event) {
        match event {
            ViewStoreTypes::Event::ModeChanged { view, .. }
            | ViewStoreTypes::Event::Destroyed { view } => {
                s.p.search.searches.remove(view);
            }
            ViewStoreTypes::Event::CursorsMoved { view }
            | ViewStoreTypes::Event::CursorsChanged { view } => {
                // Delayed events could be from already removed views.
                let Some(vse) = s.view_store.get_mut(view) else { return };

                let searching = vse.mode() == ViewStoreTypes::Mode::Search;
                let offset = vse.cursors.list.first().map(|c| c.offset);
                let Some(search) = vse.search.as_mut() else { return };

                if searching
                    && let Some(offset) = offset
                    && let Some(index) =
                        search.matches.partition_point(|m| m.start <= offset).checked_sub(1)
                {
                    let m = &search.matches[index];
                    if offset == m.start || offset < m.end {
                        search.last_match = Some(index);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn search(s: &mut State, view: ViewId, doc: DocId, bounds: Option<Vec<(usize, usize)>>) {
        let Some(request) = Self::init(s, view, doc) else { return };
        let prompt = if bounds.is_some() { "Search selection: " } else { "Search doc: " };

        let callback = Box::new(move |s: &mut State, query: Option<String>| {
            let Some(query) = query else {
                Self::finalize(s, request);
                return;
            };

            if query.is_empty() || !Self::is_current(s, request) {
                Self::finalize(s, request);
                return;
            }

            // TODO: somehow don't require copying the entire doc anymore?
            let text = s.doc_store.get(&request.doc).unwrap().doc.data.slice(..);
            let tx = s.async_fcmd_tx.clone();
            tokio::task::spawn_blocking(move || {
                let regex = match Regex::new(&query) {
                    Ok(regex) => regex,
                    Err(_) => {
                        let _ = tx.send(FCmd::Search(SearchCmd::Failed {
                            request,
                            message: "Invalid regex".to_string(),
                        }));

                        return;
                    }
                };

                let matches = regex
                    .find_iter(&text)
                    .filter_map(|result| {
                        let m = match result {
                            Ok(m) => m,
                            Err(err) => return Some(Err(err)),
                        };
                        let (start, end) = (m.start(), m.end());
                        if bounds
                            .as_ref()
                            .is_some_and(|b| !b.iter().any(|&(s, e)| start >= s && end <= e))
                        {
                            return None;
                        }

                        Some(Ok(SearchMatch { start, end, replacement: None }))
                    })
                    .collect();

                let cmd = match matches {
                    Ok(matches) => SearchCmd::SearchCompletion { request, query, matches },
                    Err(err) => {
                        SearchCmd::Failed { request, message: format!("Search failed: {err}") }
                    }
                };
                let _ = tx.send(FCmd::Search(cmd));
            });
        });

        MpProtocol::prompt(s, prompt.to_string(), None, callback, None);
    }

    pub fn search_completion(
        s: &mut State, request: SearchRequest, query: String, matches: Vec<SearchMatch>,
    ) {
        if !Self::finalize(s, request) {
            return;
        }

        let first = matches.first().map(|m| m.start);
        let vse = s.view_store.get_mut(&request.view).unwrap();
        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();
        vse.search = Some(SearchState::new(query, matches, request.revision));

        ActionProtocol::push_view_mode(s, request.view, ViewStoreTypes::Mode::Search);

        if let Some(first) = first {
            ActionProtocol::remove_cursors(s, request.view, &offsets);
            ActionProtocol::create_cursors_at_offsets(s, request.view, vec![first]);
        } else {
            util::mp::message(s, "No matches found".to_string(), Duration::from_secs(3));
        }
    }

    pub fn navigate(s: &mut State, view: ViewId, forward: bool) {
        let vse = s.view_store.get_mut(&view).unwrap();

        if vse.mode() != ViewStoreTypes::Mode::Search {
            return;
        }

        let search = vse.search.as_mut().unwrap();

        let matches = &search.matches;
        if matches.is_empty() {
            return;
        }

        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();
        let offset = offsets.first().copied().unwrap_or(0);

        let target = if forward {
            let next = matches.partition_point(|m| m.start <= offset);
            matches.get(next).unwrap_or(&matches[0]).start
        } else {
            let next = matches.partition_point(|m| m.start < offset);
            matches[next.checked_sub(1).unwrap_or(matches.len() - 1)].start
        };

        ActionProtocol::remove_cursors(s, view, &offsets);
        ActionProtocol::create_cursors_at_offsets(s, view, vec![target]);
    }

    pub fn replace(s: &mut State, view: ViewId, doc: DocId, all: bool) {
        if s.p.mp.kind == MpKind::Prompt || s.index.view_to_doc(view) != Some(doc) {
            return;
        }

        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if dse.read_only || vse.mode() != ViewStoreTypes::Mode::Search {
            return;
        }

        let search = vse.search.as_ref().unwrap();

        if search.revision != dse.doc.revision {
            Self::close(s, view);
            util::mp::message(s, "Document changed".to_string(), Duration::from_secs(3));
            return;
        }

        if search.matches.is_empty() {
            util::mp::message(s, "No matches found".to_string(), Duration::from_secs(3));
            return;
        }

        if s.p.search.searches.contains_key(&view) {
            util::mp::message(s, "Replacement in progress".to_string(), Duration::from_secs(3));
            return;
        }

        let index = if all {
            None
        } else {
            let offset = vse.cursors.list.first().map_or(0, |c| c.offset);
            let next = search.matches.partition_point(|m| {
                if m.start == m.end { m.start < offset } else { m.end <= offset }
            });

            Some(if next < search.matches.len() { next } else { 0 })
        };

        if search.replacement.is_some() {
            Self::apply(s, view, doc, index);
            return;
        }

        let query = search.query.clone();
        let matches = search.matches.clone();
        let request = Self::init(s, view, doc).unwrap();

        let callback = Box::new(move |s: &mut State, replacement: Option<String>| {
            let Some(replacement) = replacement else {
                Self::finalize(s, request);
                return;
            };

            Self::compute_replacement(s, request, query, matches, replacement, index);
        });

        let prompt = if all { "Replace all with: " } else { "Replace with: " };
        MpProtocol::prompt(s, prompt.to_string(), None, callback, None);
    }

    pub fn replace_completion(
        s: &mut State, request: SearchRequest, matches: Vec<SearchMatch>, replacement: String,
        index: Option<usize>,
    ) {
        if !Self::finalize(s, request) || s.doc_store.get(&request.doc).unwrap().read_only {
            return;
        }

        let vse = s.view_store.get_mut(&request.view).unwrap();
        if vse.mode() != ViewStoreTypes::Mode::Search {
            return;
        }

        let search = vse.search.as_mut().unwrap();

        search.matches = matches;
        search.replacement = Some(replacement);

        Self::apply(s, request.view, request.doc, index);
    }

    pub fn close(s: &mut State, view: ViewId) {
        s.p.search.searches.remove(&view);

        let vse = s.view_store.get_mut(&view).unwrap();

        vse.search = None;
        if vse.mode() == ViewStoreTypes::Mode::Search {
            ActionProtocol::pop_view_mode(s, view);
        } else {
            vse.modes.retain(|mode| *mode != ViewStoreTypes::Mode::Search);
        }
    }

    fn init(s: &mut State, view: ViewId, doc: DocId) -> Option<SearchRequest> {
        if s.p.mp.kind == MpKind::Prompt || s.index.view_to_doc(view) != Some(doc) {
            return None;
        }

        let request_id = s.p.search.next_id;
        s.p.search.next_id += 1;

        let revision = s.doc_store.get(&doc).unwrap().doc.revision;

        let request = SearchRequest { request_id, view, doc, revision };
        s.p.search.searches.insert(view, request);

        Some(request)
    }

    fn compute_replacement(
        s: &mut State, request: SearchRequest, query: String, mut matches: Vec<SearchMatch>,
        replacement: String, index: Option<usize>,
    ) {
        if !Self::is_current(s, request) || s.doc_store.get(&request.doc).unwrap().read_only {
            Self::finalize(s, request);
            return;
        }

        // TODO: somehow don't require copying the entire doc anymore?
        let data = s.doc_store.get(&request.doc).unwrap().doc.data.slice(..);
        let tx = s.async_fcmd_tx.clone();
        tokio::task::spawn_blocking(move || {
            let regex = match Regex::new(&query) {
                Ok(regex) => regex,
                Err(_) => {
                    let _ = tx.send(FCmd::Search(SearchCmd::Failed {
                        request,
                        message: "Invalid regex".to_string(),
                    }));
                    return;
                }
            };

            // Expand every match once.
            for caps in regex.captures_iter(data.as_str()) {
                let caps = match caps {
                    Ok(caps) => caps,
                    Err(err) => {
                        let _ = tx.send(FCmd::Search(SearchCmd::Failed {
                            request,
                            message: format!("Replacement failed: {err}"),
                        }));
                        return;
                    }
                };

                let m = caps.get(0).unwrap();
                if let Ok(idx) =
                    matches.binary_search_by_key(&(m.start(), m.end()), |m| (m.start, m.end))
                {
                    let mut text = String::new();
                    caps.expand(&replacement, &mut text);

                    matches[idx].replacement = Some(text);
                }
            }

            if matches.iter().any(|m| m.replacement.is_none()) {
                let _ = tx.send(FCmd::Search(SearchCmd::Failed {
                    request,
                    message: "Search matches changed".to_string(),
                }));
                return;
            }

            let _ = tx.send(FCmd::Search(SearchCmd::ReplaceCompletion {
                request,
                matches,
                replacement,
                index,
            }));
        });
    }

    fn apply(s: &mut State, view: ViewId, doc: DocId, index: Option<usize>) {
        let vse = s.view_store.get_mut(&view).unwrap();
        let search = vse.search.as_mut().unwrap();

        if search.matches.is_empty() {
            return;
        }

        if search.matches.windows(2).any(|pair| pair[0].end > pair[1].start) {
            util::mp::message(
                s,
                "Overlapping matches can't be replaced".to_string(),
                Duration::from_secs(3),
            );
            return;
        }

        let mut matches = std::mem::take(&mut search.matches);
        let first = index.unwrap_or(0);
        let last = index.map_or(matches.len(), |idx| idx + 1);
        let replaced_at = matches[first].start;

        let mut removed = 0;
        let mut inserted = 0;

        ActionProtocol::start_commit(s, doc);
        for m in matches.drain(first..last).rev() {
            let text = m.replacement.unwrap();
            removed = m.end - m.start;
            inserted = text.len();

            ActionProtocol::remove(s, view, m.start, removed);
            if !text.is_empty() {
                ActionProtocol::insert_at(s, view, text, m.start);
            }
        }
        ActionProtocol::end_commit(s, doc);

        // Replace all, nothing to do anymore.
        if index.is_none() {
            Self::close(s, view);
            return;
        }

        // Shift matches after the replaced match.
        for m in &mut matches[first..] {
            m.start = m.start - removed + inserted;
            m.end = m.end - removed + inserted;
        }

        // Has to be fetched again for the borrow checker to be happy.
        let vse = s.view_store.get_mut(&view).unwrap();
        let revision = s.doc_store.get(&doc).unwrap().doc.revision;
        let search = vse.search.as_mut().unwrap();

        let next = (!matches.is_empty()).then(|| first % matches.len());
        let offset = next.map_or(replaced_at, |index| matches[index].start);

        search.matches = matches;
        search.revision = revision;
        search.last_match = next;

        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();
        ActionProtocol::remove_cursors(s, view, &offsets);
        ActionProtocol::create_cursors_at_offsets(s, view, vec![offset]);
    }

    fn is_current(s: &State, request: SearchRequest) -> bool {
        s.p.search.searches.get(&request.view) == Some(&request)
            && s.index.view_to_doc(request.view) == Some(request.doc)
            && s.doc_store.get(&request.doc).unwrap().doc.revision == request.revision
    }

    fn finalize(s: &mut State, request: SearchRequest) -> bool {
        let current = Self::is_current(s, request);

        if s.p.search.searches.get(&request.view) == Some(&request) {
            s.p.search.searches.remove(&request.view);
        }

        current
    }
}
