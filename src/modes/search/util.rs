use fancy_regex::Regex;
use piece_table::Slice;

use crate::{
    debug_panic::debug_panic,
    decorators::SearchDecorator,
    fed::FCmd,
    protocols::{action::ActionCmd, mp::MpCmd},
    state::{DocId, State, ViewId, ViewStoreTypes},
};

pub fn start_search(view: ViewId, doc: DocId, bounds: Option<Vec<(usize, usize)>>) -> Vec<FCmd> {
    let prompt = if bounds.is_some() {
        "Search selection: ".to_string()
    } else {
        "Search doc: ".to_string()
    };

    let pcallback = Box::new(move |state: &State, query: String| {
        if query.is_empty() {
            return;
        }

        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return;
        };

        // TODO: somehow don't require copying the entire doc anymore?
        let text = dse.doc.data.slice(0..dse.doc.data.len());
        let face = state.theme.search_match;
        let async_fcmd_tx = state.async_fcmd_tx.clone();
        tokio::spawn(async move {
            // Delay actually processing the regex into an asynchronous task to
            // not block the main loop.
            let Ok(regex) = Regex::new(&query) else {
                // TODO: display regex building error.
                return;
            };

            let mut matches: Vec<(usize, usize)> = Vec::new();
            for m in regex.find_iter(&text) {
                if let Ok(m) = m {
                    let (start, end) = (m.start(), m.end());

                    if let Some(ref b) = bounds
                        && !b.iter().any(|&(s, e)| start >= s && end <= e)
                    {
                        continue;
                    }

                    matches.push((start, end));
                }
            }

            if matches.is_empty() {
                return;
            }

            let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::SetViewDecorator {
                view,
                id: ViewStoreTypes::DecorationId::Search,
                dec: Box::new(SearchDecorator::new(query, matches, face)),
            }));
            let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::PushViewMode {
                view,
                mode: ViewStoreTypes::Mode::Search,
            }));
        });
    });

    vec![FCmd::Mp(MpCmd::Prompt { prompt, initial_text: None, pcallback, tx: None })]
}
