pub mod backend;
pub mod cli;
mod enforce;
pub mod layout;
pub mod models;
pub mod monitor;
mod report;
pub mod state;

pub use backend::{
    BackendError, ConfiguredBackend, DisplayBackend, InMemoryBackend, WindowGeometryChange,
    X11Backend,
};
pub use layout::{
    build_enforcement_plan, EnforcementMode, EnforcementPlan, LayoutOperation, LayoutPolicy,
    UnmanagedWindowsPolicy,
};
pub use models::{DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};
pub use monitor::DisplayMonitor;
pub use state::DisplayState;
