use std::time::Duration;

use fancy_regex::Regex;

use crate::decorators::SearchDecorator;
use crate::decorators::SearchMatch;
use crate::fed::FCmd;
use crate::protocols::action::ActionProtocol;
use crate::protocols::mp::MpKind;
use crate::protocols::mp::MpProtocol;
use crate::protocols::search::SearchCmd;
use crate::protocols::search::SearchRequest;
use crate::state::DocId;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
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

                let offset = vse.cursors.list.first().map(|c| c.offset);
                if let Some(decorator) = vse
                    .decs
                    .decorators
                    .get_mut(&ViewStoreTypes::DecorationId::Search)
                    .and_then(|dec| dec.any_mut().downcast_mut::<SearchDecorator>())
                    && decorator.replaced_at != offset
                {
                    decorator.replaced_at = None;
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

            let dse = s.doc_store.get(&doc).unwrap();

            // TODO: somehow don't require copying the entire doc anymore?
            let text = dse.doc.data.slice(..);
            let tx = s.async_fcmd_tx.clone();
            tokio::spawn(async move {
                // Delay actually processing the regex into an asynchronous task
                // to not block the main loop.
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
                    .flatten()
                    .filter_map(|m| {
                        let (start, end) = (m.start(), m.end());
                        if bounds
                            .as_ref()
                            .is_some_and(|b| !b.iter().any(|&(s, e)| start >= s && end <= e))
                        {
                            return None;
                        }

                        Some(SearchMatch { start, end, replacement: None })
                    })
                    .collect();

                let _ =
                    tx.send(FCmd::Search(SearchCmd::SearchCompletion { request, query, matches }));
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

        if matches.is_empty() {
            util::mp::message(s, "No matches found".to_string(), Duration::from_secs(3));
            return;
        }

        let decorator =
            SearchDecorator::new(query, matches, s.theme.search_match, request.revision);
        ActionProtocol::set_view_decorator(
            s,
            request.view,
            ViewStoreTypes::DecorationId::Search,
            Box::new(decorator),
        );
        ActionProtocol::push_view_mode(s, request.view, ViewStoreTypes::Mode::Search);
    }

    pub fn navigate(s: &mut State, view: ViewId, forward: bool) {
        let vse = s.view_store.get_mut(&view).unwrap();

        if vse.mode() != ViewStoreTypes::Mode::Search {
            return;
        }

        let decorator = vse.decs.decorators.get_mut(&ViewStoreTypes::DecorationId::Search).unwrap();
        let decorator = decorator.any_mut().downcast_mut::<SearchDecorator>().unwrap();

        let matches = &decorator.matches;
        if matches.is_empty() {
            return;
        }

        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();
        let offset = offsets.first().copied().unwrap_or(0);

        let target = if forward {
            let after_replace = decorator.replaced_at.take() == Some(offset);
            let next = matches.partition_point(|m| {
                if after_replace { m.start < offset } else { m.start <= offset }
            });
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

        let decorator = vse.decs.decorators.get(&ViewStoreTypes::DecorationId::Search).unwrap();
        let decorator = decorator.any().downcast_ref::<SearchDecorator>().unwrap();

        if decorator.revision != dse.doc.revision {
            util::mp::message(s, "Document changed".to_string(), Duration::from_secs(3));
            return;
        }

        if decorator.matches.is_empty() {
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
            let next = decorator.matches.partition_point(|m| {
                if m.start == m.end { m.start < offset } else { m.end <= offset }
            });

            Some(if next < decorator.matches.len() { next } else { 0 })
        };

        if decorator.replacement.is_some() {
            Self::apply(s, view, doc, index);
            return;
        }

        let query = decorator.query.clone();
        let matches = decorator.matches.clone();
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

        let decorator = vse.decs.decorators.get_mut(&ViewStoreTypes::DecorationId::Search).unwrap();
        let decorator = decorator.any_mut().downcast_mut::<SearchDecorator>().unwrap();

        decorator.matches = matches;
        decorator.replacement = Some(replacement);

        Self::apply(s, request.view, request.doc, index);
    }

    pub fn close(s: &mut State, view: ViewId) {
        s.p.search.searches.remove(&view);

        let vse = s.view_store.get(&view).unwrap();

        if vse.mode() != ViewStoreTypes::Mode::Search {
            return;
        }

        ActionProtocol::remove_view_decorator(s, view, ViewStoreTypes::DecorationId::Search);
        ActionProtocol::pop_view_mode(s, view);
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
        tokio::spawn(async move {
            // Delay actually processing the regex into an asynchronous
            // task to not block the main loop.
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
            for caps in regex.captures_iter(&data) {
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
        let decorator = vse.decs.decorators.get_mut(&ViewStoreTypes::DecorationId::Search).unwrap();
        let decorator = decorator.any_mut().downcast_mut::<SearchDecorator>().unwrap();

        if decorator.matches.is_empty() {
            return;
        }

        if decorator.matches.windows(2).any(|pair| pair[0].end > pair[1].start) {
            util::mp::message(
                s,
                "Overlapping matches can't be replaced".to_string(),
                Duration::from_secs(3),
            );
            return;
        }

        let mut matches = std::mem::take(&mut decorator.matches);
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
        let decorator = vse.decs.decorators.get_mut(&ViewStoreTypes::DecorationId::Search).unwrap();
        let decorator = decorator.any_mut().downcast_mut::<SearchDecorator>().unwrap();

        decorator.matches = matches;
        decorator.revision = revision;
        decorator.replaced_at = Some(replaced_at);

        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();
        ActionProtocol::remove_cursors(s, view, &offsets);
        ActionProtocol::create_cursors_at_offsets(s, view, vec![replaced_at]);
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
