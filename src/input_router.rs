use crossterm::event::{Event, MouseButton, MouseEventKind};

use crate::{
    fed::FCmd,
    input_handler::{KeyInputHandler, MouseInputHandler, PasteInputHandler, ResizeInputHandler},
    modes::{
        dir::{DirKeyInput, DirMouseInput},
        insert::{InsertKeyInput, InsertMouseInput},
        mp::{MpKeyInput, MpMouseInput},
        normal::{NormalKeyInput, NormalMouseInput},
        search::{SearchKeyInput, SearchMouseInput},
        visual::{VisualKeyInput, VisualMouseInput},
    },
    protocols::{mp::MpResizeInput, screen::ScreenResizeInput, view::ViewResizeInput},
    state::State,
};

pub struct InputRouter {
    key_handlers: Vec<Box<dyn KeyInputHandler>>,
    mouse_handlers: Vec<Box<dyn MouseInputHandler>>,
    paste_handlers: Vec<Box<dyn PasteInputHandler>>,
    resize_handlers: Vec<Box<dyn ResizeInputHandler>>,
}

impl InputRouter {
    pub fn exec(&mut self, state: &mut State, event: Event) -> Vec<FCmd> {
        match event {
            Event::Key(key) => {
                for handler in &mut self.key_handlers {
                    if let Some(cmds) = handler.key(state, &key) {
                        return cmds;
                    }
                }

                Vec::new()
            }
            Event::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                    && let Some(w) = state.workspace.get_window((mouse.column, mouse.row).into())
                {
                    state.workspace.active_window = Some(w);
                }

                for handler in &mut self.mouse_handlers {
                    if let Some(cmds) = handler.mouse(state, &mouse) {
                        return cmds;
                    }
                }

                Vec::new()
            }
            Event::Paste(str) => {
                for handler in &mut self.paste_handlers {
                    if let Some(cmds) = handler.paste(state, &str) {
                        return cmds;
                    }
                }

                Vec::new()
            }
            Event::Resize(width, height) => {
                let mut cmds = Vec::new();
                for handler in &mut self.resize_handlers {
                    cmds.extend(handler.resize(state, (width, height)));
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
            Box::new(SearchKeyInput::new()),
            Box::new(VisualKeyInput::new()),
            Box::new(DirKeyInput::new()),
            Box::new(InsertKeyInput::new()),
            Box::new(NormalKeyInput::new()),
        ];

        let mut mouse_handlers: Vec<Box<dyn MouseInputHandler>> = vec![
            Box::new(MpMouseInput {}),
            Box::new(SearchMouseInput {}),
            Box::new(VisualMouseInput {}),
            Box::new(DirMouseInput {}),
            Box::new(InsertMouseInput {}),
            Box::new(NormalMouseInput {}),
        ];
        let mut paste_handlers: Vec<Box<dyn PasteInputHandler>> = vec![];
        let mut resize_handlers: Vec<Box<dyn ResizeInputHandler>> = vec![
            Box::new(ScreenResizeInput {}),
            Box::new(MpResizeInput {}),
            Box::new(ViewResizeInput {}),
        ];

        key_handlers.sort_by(|a, b| a.priority().cmp(&b.priority()));
        mouse_handlers.sort_by(|a, b| a.priority().cmp(&b.priority()));
        paste_handlers.sort_by(|a, b| a.priority().cmp(&b.priority()));
        resize_handlers.sort_by(|a, b| a.priority().cmp(&b.priority()));

        Self { key_handlers, mouse_handlers, paste_handlers, resize_handlers }
    }
}
