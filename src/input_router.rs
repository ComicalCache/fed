use crossterm::event::Event;

use crate::fed::FCmd;
use crate::modes::dir::DirKeyInput;
use crate::modes::dir::DirMouseInput;
use crate::modes::insert::InsertKeyInput;
use crate::modes::insert::InsertMouseInput;
use crate::modes::insert::InsertPasteInput;
use crate::modes::manage::ManageKeyInput;
use crate::modes::manage::ManageMouseInput;
use crate::modes::mp::MpKeyInput;
use crate::modes::mp::MpMouseInput;
use crate::modes::mp::MpPasteInput;
use crate::modes::normal::NormalKeyInput;
use crate::modes::normal::NormalMouseInput;
use crate::modes::normal::NormalPasteInput;
use crate::modes::picker::PickerKeyInput;
use crate::modes::picker::PickerMouseInput;
use crate::modes::picker::PickerPasteInput;
use crate::modes::read::ReadKeyInput;
use crate::modes::read::ReadMouseInput;
use crate::modes::search::SearchKeyInput;
use crate::modes::search::SearchMouseInput;
use crate::modes::visual::VisualKeyInput;
use crate::modes::visual::VisualMouseInput;
use crate::modes::visual::VisualPasteInput;
use crate::protocols::mp::MpResizeInput;
use crate::protocols::picker::PickerResizeInput;
use crate::protocols::screen::ScreenResizeInput;
use crate::protocols::view::ViewResizeInput;
use crate::state::State;
use crate::types::KeyInputHandler;
use crate::types::MouseInputHandler;
use crate::types::PasteInputHandler;
use crate::types::ResizeInputHandler;

pub struct InputRouter {
    key_handlers: Vec<Box<dyn KeyInputHandler>>,
    mouse_handlers: Vec<Box<dyn MouseInputHandler>>,
    paste_handlers: Vec<Box<dyn PasteInputHandler>>,
    resize_handlers: Vec<Box<dyn ResizeInputHandler>>,
}

impl InputRouter {
    pub fn exec(&mut self, s: &State, event: Event) -> Vec<FCmd> {
        match event {
            Event::Key(key) => {
                for handler in &mut self.key_handlers {
                    if let Some(cmds) = handler.key(s, &key) {
                        return cmds;
                    }
                }

                Vec::new()
            }
            Event::Mouse(mouse) => {
                let Some(window) = s.workspace.get_window((mouse.column, mouse.row).into()) else {
                    return Vec::new();
                };

                for handler in &mut self.mouse_handlers {
                    if let Some(cmds) = handler.mouse(s, window, mouse) {
                        return cmds;
                    }
                }

                Vec::new()
            }
            Event::Paste(str) => {
                for handler in &mut self.paste_handlers {
                    if let Some(cmds) = handler.paste(s, &str) {
                        return cmds;
                    }
                }

                Vec::new()
            }
            Event::Resize(width, height) => {
                let mut cmds = Vec::new();
                for handler in &mut self.resize_handlers {
                    cmds.extend(handler.resize(s, (width, height)));
                }

                cmds
            }
            _ => Vec::new(),
        }
    }
}

impl Default for InputRouter {
    fn default() -> Self {
        let mut key_handlers: Vec<Box<dyn KeyInputHandler>> = vec![
            Box::new(MpKeyInput::new()),
            Box::new(PickerKeyInput::new()),
            Box::new(SearchKeyInput::new()),
            Box::new(VisualKeyInput::new()),
            Box::new(ReadKeyInput::new()),
            Box::new(ManageKeyInput::new()),
            Box::new(DirKeyInput::new()),
            Box::new(InsertKeyInput::new()),
            Box::new(NormalKeyInput::new()),
        ];

        let mut mouse_handlers: Vec<Box<dyn MouseInputHandler>> = vec![
            Box::new(MpMouseInput {}),
            Box::new(PickerMouseInput {}),
            Box::new(SearchMouseInput {}),
            Box::new(VisualMouseInput {}),
            Box::new(ReadMouseInput {}),
            Box::new(ManageMouseInput {}),
            Box::new(DirMouseInput {}),
            Box::new(InsertMouseInput {}),
            Box::new(NormalMouseInput {}),
        ];
        let mut paste_handlers: Vec<Box<dyn PasteInputHandler>> = vec![
            Box::new(MpPasteInput {}),
            Box::new(PickerPasteInput {}),
            Box::new(VisualPasteInput {}),
            Box::new(InsertPasteInput {}),
            Box::new(NormalPasteInput {}),
        ];
        let mut resize_handlers: Vec<Box<dyn ResizeInputHandler>> = vec![
            Box::new(ScreenResizeInput {}),
            Box::new(PickerResizeInput {}),
            Box::new(MpResizeInput {}),
            Box::new(ViewResizeInput {}),
        ];

        key_handlers.sort_by_key(|a| a.priority());
        mouse_handlers.sort_by_key(|a| a.priority());
        paste_handlers.sort_by_key(|a| a.priority());
        resize_handlers.sort_by_key(|a| a.priority());

        Self { key_handlers, mouse_handlers, paste_handlers, resize_handlers }
    }
}
