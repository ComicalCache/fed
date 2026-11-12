use unicode_width::UnicodeWidthStr;

pub use crate::modes::read::setup::dmode;
pub use crate::modes::read::setup::vmode;
use crate::piece_table::PieceTable;
use crate::protocols::action::ActionProtocol;
use crate::protocols::keybind::KeybindProtocol;
use crate::state::DocStoreTypes;
use crate::state::State;
use crate::types::KeymapHelpBinding;

pub fn setup(s: &mut State) {
    let max_key_width = s
        .keymap_help
        .iter()
        .flat_map(|keymap| &keymap.bindings)
        .map(|b| b.sequence.width())
        .max()
        .unwrap_or(0)
        .max(15);
    let mut text = String::new();
    for keymap in s.keymap_help.iter().filter(|keymap| !keymap.bindings.is_empty()) {
        if !text.is_empty() {
            text.push('\n');
        }

        text.push_str(&format!("[{}]\n", keymap.mode.to_uppercase()));
        for KeymapHelpBinding { sequence, command, synopsis } in &keymap.bindings {
            let padding = " ".repeat(max_key_width - sequence.width() + 1);
            text.push_str(&format!("  {sequence}{padding}Cmd::{command}"));

            if !synopsis.is_empty() {
                text.push_str("  ");
                text.push_str(synopsis);
            }

            text.push('\n');
        }
    }

    let doc = s.create_doc(None).unwrap();
    s.p.keybind.doc = Some(doc);
    s.doc_store.get_mut(&doc).unwrap().doc.data = PieceTable::from(text);
    ActionProtocol::set_doc_mode(s, doc, DocStoreTypes::Mode::Keybind);
    KeybindProtocol::decorate(s);
}
