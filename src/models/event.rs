use super::{DisplayOutput, Rect, WindowId, WindowInfo};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayEvent {
    Reset,
    OutputConnected(DisplayOutput),
    OutputDisconnected { name: String },
    OutputGeometryChanged { name: String, geometry: Rect },
    WindowMapped(WindowInfo),
    WindowUnmapped(WindowId),
    WindowConfigured { id: WindowId, geometry: Rect },
    WindowRaised(WindowId),
    FocusChanged(Option<WindowId>),
}
