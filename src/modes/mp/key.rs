use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    fed::FCmd,
    modes::mp::{cmd::Cmd, keymap},
    protocols::{
        action::ActionCmd,
        mp::{MpCmd, MpKind},
    },
    state::{State, ViewId},
    types::{KeyChord, KeyInputHandler, KeyInputPriority, Keymap, ParseResult},
};

pub struct MpKeyInput {
    keymap: Keymap<Cmd>,
    pending_keys: Vec<KeyChord>,

    last_view: Option<ViewId>,
}

impl MpKeyInput {
    pub fn new() -> Self {
        let keymap = keymap::keymap();
        Self { keymap, pending_keys: Vec::new(), last_view: None }
    }

    fn exec(s: &State, cmd: Cmd) -> Vec<FCmd> {
        let id = s.p.mp.id;
        let view = s.p.mp.view;

        match cmd {
            Cmd::Input(text) => vec![FCmd::Action(ActionCmd::Insert { view, text })],
            Cmd::Move(motion) => {
                vec![FCmd::Action(ActionCmd::MoveCursors { view, motion, move_anchor: true })]
            }
            Cmd::Backspace => vec![FCmd::Action(ActionCmd::Backspace { view })],
            Cmd::Delete => vec![FCmd::Action(ActionCmd::Delete { view })],
            Cmd::Submit => vec![FCmd::Mp(MpCmd::Submit)],
            Cmd::Close => vec![FCmd::Mp(MpCmd::Close { id })],
        }
    }
}

impl KeyInputHandler for MpKeyInput {
    fn priority(&self) -> KeyInputPriority { KeyInputPriority::MpMode }

    fn key(&mut self, s: &State, event: &KeyEvent) -> Option<Vec<FCmd>> {
        if s.p.mp.window.is_none() || s.workspace.active_window != s.p.mp.window {
            return None;
        }
        if s.p.mp.kind != MpKind::Prompt {
            return Some(Vec::new());
        }

        if self.last_view != Some(s.p.mp.view) {
            self.pending_keys.clear();

            self.last_view = Some(s.p.mp.view);
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
