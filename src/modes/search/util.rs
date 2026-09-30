use fancy_regex::Regex;
use piece_table::Slice;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    debug_panic::debug_panic,
    decorators::SearchDecorator,
    protocols::{action::ActionCommand, mini_buffer::MiniBufferCommand},
    state::{DocId, StateLock, ViewId, ViewStoreTypes},
};

pub fn start_search(
    state_lock: StateLock, view: ViewId, doc: DocId, bounds: Option<Vec<(usize, usize)>>,
    action_tx: UnboundedSender<ActionCommand>, mini_buffer_tx: UnboundedSender<MiniBufferCommand>,
) {
    let prompt = if bounds.is_some() {
        "Search selection: ".to_string()
    } else {
        "Search doc: ".to_string()
    };

    let (id_tx, _) = oneshot::channel();
    let (res_tx, res_rx) = oneshot::channel();
    let _ = mini_buffer_tx.send(MiniBufferCommand::Prompt {
        prompt,
        initial_text: None,
        id_tx,
        res_tx,
    });

    tokio::spawn(async move {
        let Ok(query) = res_rx.await else { return };
        if query.is_empty() {
            return;
        }

        execute_search(state_lock, view, doc, query, bounds, action_tx);
    });
}

pub fn execute_search(
    state_lock: StateLock, view: ViewId, doc: DocId, query: String,
    bounds: Option<Vec<(usize, usize)>>, action_tx: UnboundedSender<ActionCommand>,
) {
    let state = state_lock.read();
    let Some(dse) = state.doc_store.get(&doc) else {
        debug_panic!();
        return;
    };

    let text = dse.doc.data.slice(0..dse.doc.data.len());
    drop(state);

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

    let mut guard = state_lock.write();
    // Fix the borrow checker.
    let state = &mut *guard;

    let Some(vse) = state.view_store.get_mut(&view) else {
        debug_panic!();
        return;
    };

    vse.decs.decorators.insert(
        ViewStoreTypes::DecorationId::Search,
        Box::new(SearchDecorator::new(query, matches, state.theme.search_match)),
    );

    drop(guard);

    let _ =
        action_tx.send(ActionCommand::PushViewMode { view, mode: ViewStoreTypes::Mode::Search });
}
