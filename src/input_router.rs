use crossterm::event::Event;

use crate::{
    fed::FCmd,
    modes::{
        dir::{DirKeyInput, DirMouseInput},
        insert::{InsertKeyInput, InsertMouseInput, InsertPasteInput},
        manage::{ManageKeyInput, ManageMouseInput},
        mp::{MpKeyInput, MpMouseInput, MpPasteInput},
        normal::{NormalKeyInput, NormalMouseInput, NormalPasteInput},
        picker::{PickerKeyInput, PickerMouseInput, PickerPasteInput},
        read::{ReadKeyInput, ReadMouseInput},
        search::{SearchKeyInput, SearchMouseInput},
        visual::{VisualKeyInput, VisualMouseInput, VisualPasteInput},
    },
    protocols::{
        mp::MpResizeInput, picker::PickerResizeInput, screen::ScreenResizeInput,
        view::ViewResizeInput,
    },
    state::State,
    types::{KeyInputHandler, MouseInputHandler, PasteInputHandler, ResizeInputHandler},
};

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
