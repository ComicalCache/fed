use crate::types::render::WindowId;

#[allow(unused)]
pub enum ManageCmd {
    Show { window: WindowId },

    Select,
    Close,
}
