pub mod backend;
pub mod cli;
pub mod models;
pub mod monitor;
pub mod state;

pub use backend::{BackendError, ConfiguredBackend, DisplayBackend, InMemoryBackend, X11Backend};
pub use models::{DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};
pub use monitor::DisplayMonitor;
pub use state::DisplayState;
