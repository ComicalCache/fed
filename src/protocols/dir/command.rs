use crate::render::WindowId;

pub enum DirCommand {
    Init,
    ReplaceWindow { window: WindowId },
    Select,
}
