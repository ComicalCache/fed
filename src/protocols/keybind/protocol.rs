use unicode_width::UnicodeWidthStr;

use crate::decorators::SpanDecorator;
use crate::modes::keybind::setup;
use crate::protocols::action::ActionProtocol;
use crate::protocols::doc_view::DocViewProtocol;
use crate::protocols::keybind::KeybindCmd;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::types::Decoration;
use crate::types::Face;
use crate::types::Span;
use crate::types::render::WindowId;

pub struct KeybindProtocol {}

impl KeybindProtocol {
    pub fn exec(s: &mut State, cmd: KeybindCmd) {
        match cmd {
            KeybindCmd::Show { window } => Self::show(s, window),
        }
    }

    pub fn show(s: &mut State, window: WindowId) {
        let doc = match s.p.keybind.doc {
            Some(doc) => doc,
            _ => {
                setup::setup(s);
                s.p.keybind.doc.unwrap()
            }
        };

        let views = s.index.doc_to_views(doc);
        if let Some(window) = views.and_then(|v| v.iter().find_map(|v| s.index.view_to_window(*v)))
        {
            ActionProtocol::set_active_window(s, Some(window));
            return;
        }

        let view = views.and_then(|v| v.first().copied());
        DocViewProtocol::show(s, doc, view, window, false, None);
    }

    pub fn decorate(s: &mut State) {
        let Some(doc) = s.p.keybind.doc else { return };
        let dse = s.doc_store.get(&doc).unwrap();

        if dse.mode != DocStoreTypes::Mode::Keybind {
            return;
        }

        let command = Face { fg: s.theme.selection.bg.or(s.theme.default.fg), ..Face::default() };
        let heading = Face { bold: Some(true), ..command };
        let sequence =
            Face { fg: s.theme.active_cursor.bg.or(s.theme.default.fg), ..Face::default() };
        let synopsis = Face { fg: s.theme.gutter.fg, italic: Some(true), ..Face::default() };

        let key_width = s
            .keymap_help
            .iter()
            .flat_map(|keymap| &keymap.bindings)
            .map(|b| b.sequence.width())
            .max()
            .unwrap_or(0)
            .max(15);
        let mut spans = Vec::new();
        let mut y = 0;
        for keymap in s.keymap_help.iter().filter(|keymap| !keymap.bindings.is_empty()) {
            if y > 0 {
                y += 1;
            }

            let start = dse.doc.data.line_start_offset(y);

            spans.push(Span {
                start,
                end: start + keymap.mode.to_uppercase().len() + 2,
                data: Decoration::Style { face: heading },
            });

            y += 1;

            for binding in &keymap.bindings {
                let start = dse.doc.data.line_start_offset(y) + 2;
                let sequence_end = start + binding.sequence.len();
                let command_start = sequence_end + key_width - binding.sequence.width() + 1;
                let command_end = command_start + "Cmd::".len() + binding.command.len();

                spans.push(Span {
                    start,
                    end: sequence_end,
                    data: Decoration::Style { face: sequence },
                });
                spans.push(Span {
                    start: command_start,
                    end: command_end,
                    data: Decoration::Style { face: command },
                });

                if !binding.synopsis.is_empty() {
                    let start = command_end + 2;
                    spans.push(Span {
                        start,
                        end: start + binding.synopsis.len(),
                        data: Decoration::Style { face: synopsis },
                    });
                }

                y += 1;
            }
        }

        ActionProtocol::set_doc_decorator(
            s,
            doc,
            DocStoreTypes::DecorationId::Span,
            Box::new(SpanDecorator::new(spans)),
        );
    }
}
