mod cell;
mod layout;
mod mode_line;
mod screen;
mod viewport;
mod workspace;

pub use cell::Cell;
pub use layout::layout;
pub use mode_line::{ModeLineConfig, ModeLineWidget};
pub use screen::Screen;
pub use viewport::Viewport;
pub use workspace::{WindowId, Workspace};

/// A trait that should be implemented by protocol renderers to render state to
/// the `Screen`.
pub trait Renderer: Send + Sync + 'static {
    /// Renders the protocol state to the `Screen` via a `Viewport`. This is ran
    /// _synchronously_ for all handlers and thus should not do heavy logic.
    fn render(&self, viewport: &mut Viewport, window: WindowId);
}
