use crate::render::WindowId;

pub enum ManageCmd {
    ReplaceWindow { window: WindowId },

    Select,
    Close,
}
