use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use crate::fed::FCmd;
use crate::modes::picker::cmd::Cmd;
use crate::modes::picker::keymap;
use crate::protocols::action::ActionCmd;
use crate::protocols::picker::PickerCmd;
use crate::state::State;
use crate::types::KeyChord;
use crate::types::KeyInputHandler;
use crate::types::KeyInputPriority;
use crate::types::Keymap;
use crate::types::ParseResult;
use crate::util::text::sanitize;

pub struct PickerKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,
}

impl PickerKeyInput {
    pub fn new() -> Self { Self { keymap: keymap::keymap(), pending_keys: Vec::new() } }

    fn exec(s: &State, cmd: Cmd) -> Vec<FCmd> {
        let view = s.p.picker.input_view;
        let doc = s.p.picker.input_doc;

        match cmd {
            Cmd::Input(text) => {
                let text = sanitize(&text, |ch| ch == '\t');
                if text.is_empty() {
                    return Vec::new();
                }

                vec![
                    FCmd::Action(ActionCmd::Insert { view, text }),
                    FCmd::Action(ActionCmd::Saved { doc }),
                    FCmd::Picker(PickerCmd::FilterList),
                ]
            }
            Cmd::Move(motion) => {
                vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })]
            }
            Cmd::Backspace => vec![
                FCmd::Action(ActionCmd::Backspace { view }),
                FCmd::Action(ActionCmd::Saved { doc }),
                FCmd::Picker(PickerCmd::FilterList),
            ],
            Cmd::Delete => {
                vec![
                    FCmd::Action(ActionCmd::Delete { view }),
                    FCmd::Action(ActionCmd::Saved { doc }),
                    FCmd::Picker(PickerCmd::FilterList),
                ]
            }
            Cmd::ChangeSelection(motion) => {
                vec![FCmd::Picker(PickerCmd::ChangeSelection(motion))]
            }
            Cmd::ScrollList(motion) => vec![FCmd::Picker(PickerCmd::ScrollList(motion))],
            Cmd::Select => vec![FCmd::Picker(PickerCmd::Select)],
            Cmd::Close => vec![FCmd::Picker(PickerCmd::Close)],
        }
    }
}

impl KeyInputHandler for PickerKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::PickerMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        if s.p.picker.window.is_none() || s.workspace.active_window != s.p.picker.window {
            return None;
        }

        let chord = KeyChord::from(event);
        self.pending_keys.push(chord);

        match self.keymap.parse(&self.pending_keys) {
            ParseResult::Exact(cmd) => {
                let cmds = Some(Self::exec(s, cmd));
                self.pending_keys.clear();

                cmds
            }
            ParseResult::Prefix => Some(Vec::new()),
            ParseResult::Invalid => {
                self.pending_keys.clear();

                // If it's a character input, insert it.
                let KeyCode::Char(ch) = event.code else { return Some(Vec::new()) };

                let modifiers = event
                    .modifiers
                    .iter_names()
                    .filter_map(|(_, m)| match m {
                        KeyModifiers::ALT => Some("A"),
                        KeyModifiers::CONTROL => Some("C"),
                        KeyModifiers::META => Some("M"),
                        KeyModifiers::SUPER => Some("S"),
                        KeyModifiers::HYPER => Some("H"),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("-");

                if modifiers.is_empty() {
                    Some(Self::exec(s, Cmd::Input(ch.to_string())))
                } else {
                    Some(Self::exec(s, Cmd::Input(format!("<{modifiers}-{ch}>"))))
                }
            }
        }
    }
}
