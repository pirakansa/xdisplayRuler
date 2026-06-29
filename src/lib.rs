pub mod backend;
pub mod cli;
pub mod layout;
pub mod models;
pub mod monitor;
pub mod state;

pub use backend::{
    BackendError, ConfiguredBackend, DisplayBackend, InMemoryBackend, OutputMode, OutputModeChange,
    OutputModeSelection, OutputRotation, WindowGeometryChange, X11Backend,
};
pub use layout::{
    build_enforcement_plan, EnforcementMode, EnforcementPlan, LayoutOperation, LayoutPolicy,
    UnmanagedWindowsPolicy,
};
pub use models::{DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};
pub use monitor::DisplayMonitor;
pub use state::DisplayState;
