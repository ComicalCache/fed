use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tree_sitter_highlight::HighlightConfiguration;
use tree_sitter_highlight::HighlightEvent;
use tree_sitter_highlight::Highlighter;

use crate::decorators::SyntaxDecorator;
use crate::fed::FCmd;
use crate::protocols::tree_sitter::TreeSitterCmd;
use crate::protocols::tree_sitter::config;
use crate::protocols::tree_sitter::pstate::TreeSitterDoc;
use crate::state::DocId;
use crate::state::DocStoreEntry;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::types::Span;
use crate::types::SyntaxHighlight;
use crate::util;

pub struct TreeSitterProtocol {}

impl TreeSitterProtocol {
    pub fn exec(s: &mut State, cmd: TreeSitterCmd) {
        match cmd {
            TreeSitterCmd::HighlightCompletion { doc, highlighter, result, r#gen, rev } => {
                Self::highlight_completion(s, doc, highlighter, result, r#gen, rev);
            }
        }
    }

    pub fn exec_doc_event(s: &mut State, event: &DocStoreTypes::Event) {
        let doc = match event {
            DocStoreTypes::Event::Created { doc }
            | DocStoreTypes::Event::Inserted { doc, .. }
            | DocStoreTypes::Event::Removed { doc, .. }
            | DocStoreTypes::Event::ModeChanged { doc, .. }
            | DocStoreTypes::Event::PathChanged { doc, .. } => *doc,
            DocStoreTypes::Event::Destroyed { doc } => {
                Self::detach(s, *doc);
                return;
            }
            DocStoreTypes::Event::ModifiedChanged { .. } | DocStoreTypes::Event::Written { .. } => {
                return;
            }
        };

        // Event may be received after the doc has already been removed again.
        if s.doc_store.get(&doc).is_some_and(Self::supported) {
            Self::update(s, doc);
        } else {
            Self::detach(s, doc);
        }
    }

    fn highlight_completion(
        s: &mut State, doc: DocId, highlighter: Box<Highlighter>,
        result: Result<Vec<Span<SyntaxHighlight>>, String>, r#gen: u64, rev: u64,
    ) {
        let Some(entry) = s.p.tree_sitter.docs.get_mut(&doc) else { return };
        if entry.r#gen != r#gen {
            return;
        }

        entry.highlighter = Some(highlighter);

        // Command may be received after the doc has already been removed again.
        let Some(dse) = s.doc_store.get_mut(&doc) else {
            Self::detach(s, doc);
            return;
        };

        if !Self::supported(dse) {
            Self::detach(s, doc);
            return;
        }

        if dse.doc.rev != rev {
            Self::update(s, doc);
            return;
        }

        match result {
            Ok(spans) => {
                dse.decs.decorators.insert(
                    DocStoreTypes::DecorationId::TreeSitter,
                    Box::new(SyntaxDecorator { spans }),
                );
            }
            Err(err) => {
                util::mp::message(s, format!("Tree-sitter error: {err}"), Duration::from_secs(3))
            }
        }
    }

    fn update(s: &mut State, doc: DocId) {
        let config =
            s.doc_store.get(&doc).and_then(|dse| dse.doc.path.as_deref()).and_then(config::get);
        let config = match config {
            Some(Ok(config)) => config,
            Some(Err(err)) => {
                util::mp::message(s, format!("Tree-sitter error: {err}"), Duration::from_secs(3));
                return;
            }
            None => {
                Self::detach(s, doc);
                return;
            }
        };

        // If the file type changed, detatch the old parser.
        if s.p.tree_sitter.docs.get(&doc).is_some_and(|e| !Arc::ptr_eq(&e.config, &config)) {
            Self::detach(s, doc);
        }

        let entry = s.p.tree_sitter.docs.entry(doc).or_insert_with(|| {
            let r#gen = s.p.tree_sitter.next_gen;
            s.p.tree_sitter.next_gen += 1;

            TreeSitterDoc {
                highlighter: Some(Box::default()),
                config: Arc::clone(&config),
                cancellation: Arc::new(AtomicUsize::new(0)),
                r#gen,
                rev: None,
            }
        });

        let dse = s.doc_store.get(&doc).unwrap();
        let rev = dse.doc.rev;

        if entry.rev == Some(rev) {
            return;
        }

        let Some(mut highlighter) = entry.highlighter.take() else {
            entry.cancellation.store(1, Ordering::Relaxed);
            return;
        };

        entry.rev = Some(rev);
        entry.cancellation.store(0, Ordering::Relaxed);

        // FIXME: use incremental rebuilding.
        let cancellation = Arc::clone(&entry.cancellation);
        let r#gen = entry.r#gen;
        let text = dse.doc.data.slice(0..dse.doc.data.len());
        let workers = Arc::clone(&s.p.tree_sitter.workers);

        let tx = s.async_fcmd_tx.clone();
        tokio::spawn(async move {
            let Ok(permit) = workers.acquire_owned().await else { return };

            let completion = tokio::task::spawn_blocking(move || {
                let _ = permit;
                let result = Self::highlight(&mut highlighter, &config, &text, &cancellation);

                (highlighter, result)
            })
            .await;

            let (highlighter, result) = completion
                .unwrap_or_else(|err| (Box::default(), Err(format!("Highlighting failed: {err}"))));

            let _ = tx.send(FCmd::TreeSitter(TreeSitterCmd::HighlightCompletion {
                doc,
                highlighter,
                result,
                r#gen,
                rev,
            }));
        });
    }

    fn detach(s: &mut State, doc: DocId) {
        if let Some(entry) = s.p.tree_sitter.docs.remove(&doc) {
            entry.cancellation.store(1, Ordering::Relaxed);
        }

        if let Some(dse) = s.doc_store.get_mut(&doc) {
            dse.decs.decorators.remove(&DocStoreTypes::DecorationId::TreeSitter);
        }
    }

    fn supported(dse: &DocStoreEntry) -> bool {
        matches!(dse.mode, DocStoreTypes::Mode::Text | DocStoreTypes::Mode::Read)
            && dse.doc.path.as_deref().is_some_and(|path| config::get(path).is_some())
    }

    fn highlight(
        highlighter: &mut Highlighter, config: &HighlightConfiguration, text: &str,
        cancellation: &AtomicUsize,
    ) -> Result<Vec<Span<SyntaxHighlight>>, String> {
        let events = highlighter
            .highlight(config, text.as_bytes(), None, Some(cancellation), |_| None)
            .map_err(|err| err.to_string())?;

        let mut stack = Vec::new();
        let mut spans: Vec<Span<SyntaxHighlight>> = Vec::new();

        for event in events {
            match event.map_err(|err| err.to_string())? {
                HighlightEvent::HighlightStart(highlight) => {
                    stack.push(SyntaxHighlight::ALL[highlight.0]);
                }
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    if start == end {
                        continue;
                    }

                    if let Some(&data) = stack.last() {
                        if let Some(last) = spans.last_mut()
                            && last.end == start
                            && last.data == data
                        {
                            last.end = end;
                        } else {
                            spans.push(Span { start, end, data });
                        }
                    }
                }
            }
        }

        Ok(spans)
    }
}
