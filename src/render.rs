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

/// A trait that can be implemented by additional rendering components to render
/// visual elements post state rendering.
pub trait Layer: Send + Sync + 'static {
    /// Modifies a rendered line of cells before drawing to the screen.
    fn apply(
        &self, state: &State, row: &mut [Cell], y: usize, scroll: ViewStoreTypes::Scroll,
        voms: &[VisualOffsetMapping], vse: &ViewStoreEntry, dse: &DocStoreEntry,
    );
}

/// A trait that should be implemented by protocol renderers to render state to
/// the `Screen`.
pub trait Renderer: Send + Sync + 'static {
    /// Renders the protocol state to the `Screen` via a `Viewport`. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    fn render(&self, state: &State, viewport: &mut Viewport, window: WindowId);
}
