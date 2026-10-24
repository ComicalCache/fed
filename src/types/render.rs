mod cell;
mod layout;
mod mode_line;
mod screen;
mod viewport;
mod workspace;
mod z_layer;

pub use cell::Cell;
pub use layout::VisualOffsetMapping;
pub use layout::layout_cells;
pub use layout::layout_vom;
pub use mode_line::ModeLineConfig;
pub use mode_line::ModeLineWidget;
pub use screen::Screen;
pub use viewport::Viewport;
pub use workspace::WindowId;
pub use workspace::Workspace;
pub use z_layer::ZLayer;

use crate::state::DocStoreEntry;
use crate::state::State;
use crate::state::ViewStoreEntry;
use crate::state::ViewStoreTypes;

pub trait Layer: Send + Sync + 'static {
    fn apply(
        &self, s: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    );
}

pub trait Renderer: Send + Sync + 'static {
    fn render(&self, s: &State, viewport: &mut Viewport, window: WindowId);
}
