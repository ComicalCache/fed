use crossterm::event::KeyEvent;

use crate::decorators::SearchDecorator;
use crate::fed::FCmd;
use crate::modes::search::cmd::Cmd;
use crate::modes::search::keymap;
use crate::protocols::action::ActionCmd;
use crate::protocols::search::SearchCmd;
use crate::state::State;
use crate::state::ViewId;
use crate::state::ViewStoreTypes;
use crate::types::KeyChord;
use crate::types::KeyInputHandler;
use crate::types::KeyInputPriority;
use crate::types::Keymap;
use crate::types::ParseResult;

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

    fn exec(s: &State, cmd: Cmd) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let doc = s.index.view_to_doc(view).unwrap();
        let dse = s.doc_store.get(&doc).unwrap();

        match cmd {
            Cmd::Move(motion) => {
                Some(vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })])
            }
            Cmd::NextMatch => Self::navigate_match(s, view, true),
            Cmd::PrevMatch => Self::navigate_match(s, view, false),
            Cmd::CursorsBegin => Self::cursors(s, view, true),
            Cmd::CursorsEnd => Self::cursors(s, view, false),
            Cmd::Replace => {
                if dse.read_only {
                    return None;
                }

                Some(vec![FCmd::Search(SearchCmd::Replace { view, doc })])
            }
            Cmd::Escape => Some(Self::escape(view)),
        }
    }

    fn navigate_match(s: &State, view: ViewId, forward: bool) -> Option<Vec<FCmd>> {
        let (_, matches, offsets) = Self::search_state(s, view)?;
        let offset = offsets.first().copied().unwrap_or(0);

        let target = if forward {
            let next = matches.partition_point(|m| m.0 <= offset);
            matches.get(next).unwrap_or(&matches[0]).0
        } else {
            let next = matches.partition_point(|m| m.0 < offset);
            matches[next.checked_sub(1).unwrap_or(matches.len() - 1)].0
        };

        Some(vec![
            FCmd::Action(ActionCmd::RemoveCursors { view, offsets }),
            FCmd::Action(ActionCmd::CreateCursorsAtOffset { view, offsets: vec![target] }),
        ])
    }

    fn cursors(s: &State, view: ViewId, front: bool) -> Option<Vec<FCmd>> {
        let mut cmds = Vec::new();

        let (_, matches, offsets) = Self::search_state(s, view)?;
        cmds.push(FCmd::Action(ActionCmd::RemoveCursors { view, offsets }));
        let offsets = matches.iter().map(|m| if front { m.0 } else { m.1 }).collect();
        cmds.push(FCmd::Action(ActionCmd::CreateCursorsAtOffset { view, offsets }));

        cmds.extend(Self::escape(view));

        Some(cmds)
    }

    fn escape(view: ViewId) -> Vec<FCmd> { vec![FCmd::Search(SearchCmd::Close { view })] }

    fn search_state(s: &State, view: ViewId) -> Option<(&str, &[(usize, usize)], Vec<usize>)> {
        let vse = s.view_store.get(&view).unwrap();
        let decorator = vse.decs.decorators.get(&ViewStoreTypes::DecorationId::Search).unwrap();
        let decorator = decorator.any().downcast_ref::<SearchDecorator>().unwrap();

        if decorator.matches.is_empty() {
            return None;
        }

        let offsets: Vec<_> = vse.cursors.list.iter().map(|c| c.offset).collect();

        Some((&decorator.query, &decorator.matches, offsets))
    }
}

impl KeyInputHandler for SearchKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::SearchMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let vse = s.view_store.get(&view).unwrap();

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
                let cmds = Some(Self::exec(s, cmd).unwrap_or_default());
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
