use crate::types::render::WindowId;

pub enum KeybindCmd {
    Show { window: WindowId },
}
