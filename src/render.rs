mod cell;
mod layout;
mod mode_line;
mod screen;
mod viewport;
mod workspace;
mod z_layer;

pub use cell::Cell;
pub use layout::{VisualOffsetMapping, layout_cells, layout_vom};
pub use mode_line::{ModeLineConfig, ModeLineWidget};
pub use screen::Screen;
pub use viewport::Viewport;
pub use workspace::{WindowId, Workspace};
pub use z_layer::ZLayer;

use crate::state::{DocStoreEntry, State, ViewStoreEntry, ViewStoreTypes};

pub trait Layer: Send + Sync + 'static {
    fn apply(
        &self, state: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    );
}

pub trait Renderer: Send + Sync + 'static {
    fn render(&self, state: &State, viewport: &mut Viewport, window: WindowId);
}
