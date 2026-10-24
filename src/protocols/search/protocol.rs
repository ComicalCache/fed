use std::time::Duration;

use fancy_regex::Regex;

use crate::decorators::SearchDecorator;
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
            SearchCmd::Replace { view, doc } => Self::replace(s, view, doc),
            SearchCmd::ReplaceCompletion { request, edits } => {
                Self::replace_completion(s, request, edits);
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
            ViewStoreTypes::Event::ModeChanged { id, .. }
            | ViewStoreTypes::Event::Destroyed { id } => {
                s.p.search.searches.remove(id);
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

                        Some((start, end))
                    })
                    .collect();

                let _ =
                    tx.send(FCmd::Search(SearchCmd::SearchCompletion { request, query, matches }));
            });
        });

        MpProtocol::prompt(s, prompt.to_string(), None, callback, None);
    }

    pub fn search_completion(
        s: &mut State, request: SearchRequest, query: String, matches: Vec<(usize, usize)>,
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

    pub fn replace(s: &mut State, view: ViewId, doc: DocId) {
        if s.p.mp.kind == MpKind::Prompt || s.index.view_to_doc(view) != Some(doc) {
            return;
        }

        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if dse.read_only || vse.mode() != ViewStoreTypes::Mode::Search {
            return;
        }

        let decorator = vse
            .decs
            .decorators
            .get(&ViewStoreTypes::DecorationId::Search)
            .and_then(|dec| dec.any().downcast_ref::<SearchDecorator>())
            .unwrap();

        if decorator.revision != dse.doc.revision {
            util::mp::message(s, "Document changed".to_string(), Duration::from_secs(3));
            return;
        }

        if decorator.matches.is_empty() {
            util::mp::message(s, "No matches found".to_string(), Duration::from_secs(3));
            return;
        }

        let query = decorator.query.clone();
        let matches = decorator.matches.clone();
        let Some(request) = Self::init(s, view, doc) else { return };
        let callback = Box::new(move |s: &mut State, replacement: Option<String>| {
            let Some(replacement) = replacement else {
                Self::finalize(s, request);
                return;
            };

            if !Self::is_current(s, request) || s.doc_store.get(&doc).unwrap().read_only {
                Self::finalize(s, request);
                return;
            }

            // TODO: somehow don't require copying the entire doc anymore?
            let data = s.doc_store.get(&doc).unwrap().doc.data.slice(..);
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

                let mut edits = Vec::new();
                for (start, end) in matches {
                    if let Ok(Some(caps)) = regex.captures(&data[start..end]) {
                        let mut text = String::new();
                        caps.expand(&replacement, &mut text);
                        edits.push((start, end - start, text));
                    }
                }
                edits.sort_by_key(|e| std::cmp::Reverse(e.0));

                let _ = tx.send(FCmd::Search(SearchCmd::ReplaceCompletion { request, edits }));
            });
        });

        MpProtocol::prompt(s, "Replace with: ".to_string(), None, callback, None);
    }

    pub fn replace_completion(
        s: &mut State, request: SearchRequest, edits: Vec<(usize, usize, String)>,
    ) {
        if !Self::finalize(s, request) || s.doc_store.get(&request.doc).unwrap().read_only {
            return;
        }

        ActionProtocol::start_commit(s, request.doc);
        for (offset, len, text) in edits {
            ActionProtocol::remove(s, request.view, offset, len);

            if !text.is_empty() {
                ActionProtocol::insert_at(s, request.view, text, offset);
            }
        }
        ActionProtocol::end_commit(s, request.doc);

        Self::close(s, request.view);
    }

    pub fn close(s: &mut State, view: ViewId) {
        s.p.search.searches.remove(&view);
        let Some(vse) = s.view_store.get(&view) else { return };

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

        let id = s.p.search.next_id;
        s.p.search.next_id += 1;

        let revision = s.doc_store.get(&doc).unwrap().doc.revision;

        let request = SearchRequest { id, view, doc, revision };
        s.p.search.searches.insert(view, request);

        Some(request)
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
