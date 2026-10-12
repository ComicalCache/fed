use crate::types::render::WindowId;

#[allow(unused)]
pub enum ManageCmd {
    ReplaceWindow { window: WindowId },

    Select,
    Close,
}
