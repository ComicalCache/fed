use crate::fed::FCmd;
use crate::modes::common;
use crate::protocols::action::ActionCmd;
use crate::state::State;
use crate::state::ViewStoreTypes;
use crate::types::PasteInputHandler;
use crate::types::PasteInputPriority;
use crate::util;

pub struct VisualPasteInput {}

impl PasteInputHandler for VisualPasteInput {
    fn priority(&self) -> PasteInputPriority { PasteInputPriority::VisualMode }

    fn paste(&mut self, s: &State, text: &str) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if !matches!(vse.mode(), ViewStoreTypes::Mode::Visual | ViewStoreTypes::Mode::VisualLine) {
            return None;
        }

        if text.is_empty() || dse.read_only {
            return Some(Vec::new());
        }

        let doc = s.index.view_to_doc(view).unwrap();
        let linewise = vse.mode() == ViewStoreTypes::Mode::VisualLine;

        let offsets = if linewise {
            util::selection::offsets(vse, dse)
        } else {
            vse.cursors
                .list
                .iter()
                .map(|c| (c.anchor.min(c.offset), c.anchor.max(c.offset)))
                .collect()
        };

        let mut cmds = vec![
            FCmd::Action(ActionCmd::EndCommit { doc }),
            FCmd::Action(ActionCmd::StartCommit { doc }),
        ];

        if linewise {
            let has_last_line = linewise && util::selection::has_last_line(vse, dse);
            for (start, end) in offsets.into_iter().rev() {
                let mut text = text.to_string();
                if (end < dse.doc.data.len() || !has_last_line) && !text.ends_with('\n') {
                    text.push('\n');
                }

                if end > start {
                    cmds.push(FCmd::Action(ActionCmd::Remove {
                        view,
                        offset: start,
                        len: end - start,
                    }));
                }
                cmds.push(FCmd::Action(ActionCmd::InsertAt { view, text, offset: start }));
            }
        } else {
            cmds.extend(common::remove_offsets(view, offsets));
            cmds.push(FCmd::Action(ActionCmd::Insert { view, text: text.to_string() }));
        }

        cmds.extend([
            FCmd::Action(ActionCmd::EndCommit { doc }),
            FCmd::Action(ActionCmd::ResetCursorAnchors { view }),
            FCmd::Action(ActionCmd::PopViewMode { view }),
        ]);

        Some(cmds)
    }
}
