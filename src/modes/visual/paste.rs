use crate::{
    fed::FCmd,
    modes::common,
    protocols::action::ActionCmd,
    state::{State, ViewStoreTypes},
    types::{PasteInputHandler, PasteInputPriority},
};

pub struct VisualPasteInput {}

impl PasteInputHandler for VisualPasteInput {
    fn priority(&self) -> PasteInputPriority { PasteInputPriority::VisualMode }

    fn paste(&mut self, s: &State, text: &str) -> Option<Vec<FCmd>> {
        let view = s.active_view()?;
        let (vse, dse) = State::vse_and_dse(&s.view_store, &s.doc_store, &s.index, view).unwrap();

        if vse.mode() != ViewStoreTypes::Mode::Visual {
            return None;
        }

        if text.is_empty() || dse.read_only {
            return Some(Vec::new());
        }

        let doc = s.index.view_to_doc(view).unwrap();
        let offsets = vse
            .cursors
            .list
            .iter()
            .map(|cursor| {
                let start = cursor.anchor.min(cursor.offset);
                let end = cursor.anchor.max(cursor.offset);

                (start, end)
            })
            .collect();

        let mut cmds = vec![
            FCmd::Action(ActionCmd::EndCommit { doc }),
            FCmd::Action(ActionCmd::StartCommit { doc }),
        ];
        cmds.extend(common::remove_offsets(view, offsets));
        cmds.extend([
            FCmd::Action(ActionCmd::Insert { view, text: text.to_string() }),
            FCmd::Action(ActionCmd::EndCommit { doc }),
            FCmd::Action(ActionCmd::ResetCursorAnchors { view }),
            FCmd::Action(ActionCmd::PopViewMode { view }),
        ]);

        Some(cmds)
    }
}
