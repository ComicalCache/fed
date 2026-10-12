use crate::types::render::WindowId;

pub enum ManageCmd {
    ReplaceWindow { window: WindowId },

    Select,
    Close,
}
