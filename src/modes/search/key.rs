use crossterm::event::KeyEvent;
use fancy_regex::Regex;
use piece_table::Slice;

use crate::{
    decorators::SearchDecorator,
    fed::FCmd,
    modes::search::{cmd::Cmd, keymap},
    protocols::{action::ActionCmd, mp::MpCmd, pstate::PState},
    state::{DocId, State, ViewId, ViewStoreTypes},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, ParseResult},
    util::debug_panic,
};

pub struct SearchKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl SearchKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), last_view: None }
    }

    fn exec(&mut self, state: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else { return None };
        let Some(doc) = state.index.view_to_doc(view) else {
            debug_panic!();
            return None;
        };
        let Some(dse) = state.doc_store.get(&doc) else {
            debug_panic!();
            return None;
        };

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::NextMatch => Self::navigate_match(state, view, true),
            Cmd::PrevMatch => Self::navigate_match(state, view, false),
            Cmd::CursorsBegin => Self::cursors(state, view, true),
            Cmd::CursorsEnd => Self::cursors(state, view, false),
            Cmd::Replace => {
                if dse.read_only {
                    return None;
                }

                Self::replace(state, view, doc)
            }
            Cmd::Escape => Some(Self::escape(view)),
        }
    }

    fn navigate_match(state: &State, view: ViewId, forward: bool) -> Option<Vec<FCmd>> {
        let Some((_, matches, offsets)) = Self::search_state(state, view) else { return None };
        let offset = offsets.first().cloned().unwrap_or(0);

        let target = if forward {
            matches.iter().find(|m| m.0 > offset).unwrap_or(&matches[0]).0
        } else {
            matches.iter().rev().find(|m| m.0 < offset).unwrap_or(matches.last().unwrap()).0
        };

        Some(vec![
            FCmd::Action(ActionCmd::RemoveCursors { view, offsets }),
            FCmd::Action(ActionCmd::CreateCursorsAtOffset { view, offsets: vec![target] }),
        ])
    }

    fn cursors(state: &State, view: ViewId, front: bool) -> Option<Vec<FCmd>> {
        let mut cmds = Vec::new();

        let Some((_, matches, offsets)) = Self::search_state(state, view) else { return None };
        cmds.push(FCmd::Action(ActionCmd::RemoveCursors { view, offsets }));
        let offsets = matches.iter().map(|m| if front { m.0 } else { m.1 }).collect();
        cmds.push(FCmd::Action(ActionCmd::CreateCursorsAtOffset { view, offsets }));

        cmds.extend(Self::escape(view));

        Some(cmds)
    }

    fn replace(state: &State, view: ViewId, doc: DocId) -> Option<Vec<FCmd>> {
        let Some((query, matches, _)) = Self::search_state(state, view) else { return None };

        let pcallback = Box::new(move |state: &mut State, _: &mut PState, replacement: String| {
            let Some(dse) = state.doc_store.get(&doc) else {
                debug_panic!();
                return;
            };

            // TODO: somehow don't require copying the entire doc anymore?
            let data = dse.doc.data.slice(0..dse.doc.data.len());
            let async_fcmd_tx = state.async_fcmd_tx.clone();
            tokio::spawn(async move {
                // Delay actually processing the regex into an asynchronous
                // task to not block the main loop.
                let Ok(regex) = Regex::new(&query) else {
                    // TODO: display error about regex.
                    return;
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

                let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::StartCommit { doc }));

                for (offset, len, text) in edits {
                    let _ =
                        async_fcmd_tx.send(FCmd::Action(ActionCmd::Remove { view, offset, len }));
                    if !text.is_empty() {
                        let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::InsertAt {
                            view,
                            text,
                            offset,
                        }));
                    }
                }

                let _ = async_fcmd_tx.send(FCmd::Action(ActionCmd::EndCommit { doc }));

                for cmd in Self::escape(view) {
                    let _ = async_fcmd_tx.send(cmd);
                }
            });
        });

        Some(vec![FCmd::Mp(MpCmd::Prompt {
            prompt: "Replace with: ".to_string(),
            initial_text: None,
            pcallback,
            tx: None,
        })])
    }

    fn escape(view: ViewId) -> Vec<FCmd> {
        vec![
            FCmd::Action(ActionCmd::RemoveViewDecorator {
                view,
                id: ViewStoreTypes::DecorationId::Search,
            }),
            FCmd::Action(ActionCmd::PopViewMode { view }),
        ]
    }

    fn search_state(
        state: &State, view: ViewId,
    ) -> Option<(String, Vec<(usize, usize)>, Vec<usize>)> {
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };
        let Some(decorator) = vse.decs.decorators.get(&ViewStoreTypes::DecorationId::Search) else {
            debug_panic!();
            return None;
        };
        let Some(decorator) = decorator.any().downcast_ref::<SearchDecorator>() else {
            debug_panic!();
            return None;
        };

        if decorator.matches.is_empty() {
            return None;
        }

        let query = decorator.query.clone();
        let matches = decorator.matches.clone();
        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();

        Some((query, matches, offsets))
    }
}

impl KeyInputHandler for SearchKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::SearchMode }

    fn key(&mut self, state: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let Some(view) = state.active_view() else {
            // No active view, just abort.
            return None;
        };
        let Some(vse) = state.view_store.get(&view) else {
            debug_panic!();
            return None;
        };

        if vse.mode() != ViewStoreTypes::Mode::Search {
            return None;
        }

        if self.last_view != Some(view) {
            self.pending_keys.clear();

            self.last_view = Some(view);
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                // Always treat entering execute as consuming the key.
                let cmds = Some(self.exec(state, cmd).unwrap_or_else(|| Vec::new()));
                self.pending_keys.clear();

                cmds
            }
            ParseResult::Prefix => Some(Vec::new()),
            ParseResult::Invalid => {
                self.pending_keys.clear();

                Some(Vec::new())
            }
        }
    }
}
