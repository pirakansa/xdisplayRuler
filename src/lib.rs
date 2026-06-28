use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}x{}+{}+{}",
            self.width, self.height, self.x, self.y
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayOutput {
    pub name: String,
    pub geometry: Rect,
    pub primary: bool,
    pub connected: bool,
}

impl DisplayOutput {
    pub fn connected(name: impl Into<String>, geometry: Rect, primary: bool) -> Self {
        Self {
            name: name.into(),
            geometry,
            primary,
            connected: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(pub u64);

impl fmt::Display for WindowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:x}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub geometry: Rect,
    pub mapped: bool,
}

impl WindowInfo {
    pub fn mapped(id: WindowId, geometry: Rect) -> Self {
        Self {
            id,
            title: None,
            app_id: None,
            geometry,
            mapped: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayEvent {
    OutputConnected(DisplayOutput),
    OutputDisconnected { name: String },
    OutputGeometryChanged { name: String, geometry: Rect },
    WindowMapped(WindowInfo),
    WindowUnmapped(WindowId),
    WindowConfigured { id: WindowId, geometry: Rect },
    WindowRaised(WindowId),
    FocusChanged(Option<WindowId>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DisplayState {
    outputs: Vec<DisplayOutput>,
    windows: Vec<WindowInfo>,
    stacking_order: Vec<WindowId>,
    focused_window: Option<WindowId>,
}

impl DisplayState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn outputs(&self) -> &[DisplayOutput] {
        &self.outputs
    }

    pub fn windows(&self) -> &[WindowInfo] {
        &self.windows
    }

    pub fn stacking_order(&self) -> &[WindowId] {
        &self.stacking_order
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.focused_window
    }

    pub fn top_window(&self) -> Option<WindowId> {
        self.stacking_order.last().copied()
    }

    pub fn apply(&mut self, event: DisplayEvent) {
        match event {
            DisplayEvent::OutputConnected(output) => self.upsert_output(output),
            DisplayEvent::OutputDisconnected { name } => {
                if let Some(output) = self.outputs.iter_mut().find(|output| output.name == name) {
                    output.connected = false;
                    output.primary = false;
                }
            }
            DisplayEvent::OutputGeometryChanged { name, geometry } => {
                if let Some(output) = self.outputs.iter_mut().find(|output| output.name == name) {
                    output.geometry = geometry;
                    output.connected = true;
                }
            }
            DisplayEvent::WindowMapped(window) => {
                let id = window.id;
                self.upsert_window(window);
                self.raise_window(id);
            }
            DisplayEvent::WindowUnmapped(id) => {
                if let Some(window) = self.windows.iter_mut().find(|window| window.id == id) {
                    window.mapped = false;
                }
                self.stacking_order.retain(|window_id| *window_id != id);
                if self.focused_window == Some(id) {
                    self.focused_window = None;
                }
            }
            DisplayEvent::WindowConfigured { id, geometry } => {
                if let Some(window) = self.windows.iter_mut().find(|window| window.id == id) {
                    window.geometry = geometry;
                }
            }
            DisplayEvent::WindowRaised(id) => self.raise_window(id),
            DisplayEvent::FocusChanged(id) => {
                self.focused_window = id.filter(|id| {
                    self.windows
                        .iter()
                        .any(|window| window.id == *id && window.mapped)
                });
            }
        }
    }

    pub fn status_report(&self) -> String {
        let mut report = String::from("display-ruler\nbackend: in-memory\n");
        report.push_str(&format!("outputs: {}\n", self.outputs.len()));

        for output in &self.outputs {
            let primary = if output.primary { " primary" } else { "" };
            let status = if output.connected {
                "connected"
            } else {
                "disconnected"
            };
            report.push_str(&format!(
                "- {}: {} {}{}\n",
                output.name, status, output.geometry, primary
            ));
        }

        report.push_str(&format!("windows: {}\n", self.windows.len()));

        for window in &self.windows {
            let mapped = if window.mapped { "mapped" } else { "unmapped" };
            report.push_str(&format!(
                "- {}: {} {}\n",
                window.id, mapped, window.geometry
            ));
        }

        report.push_str(&format!(
            "focused: {}\n",
            self.focused_window
                .map_or_else(|| "none".to_string(), |id| id.to_string())
        ));
        report.push_str(&format!(
            "top: {}\n",
            self.top_window()
                .map_or_else(|| "none".to_string(), |id| id.to_string())
        ));

        report
    }

    fn upsert_output(&mut self, output: DisplayOutput) {
        if output.primary {
            for existing in &mut self.outputs {
                existing.primary = false;
            }
        }

        if let Some(existing) = self
            .outputs
            .iter_mut()
            .find(|existing| existing.name == output.name)
        {
            *existing = output;
            return;
        }

        self.outputs.push(output);
    }

    fn upsert_window(&mut self, window: WindowInfo) {
        if let Some(existing) = self
            .windows
            .iter_mut()
            .find(|existing| existing.id == window.id)
        {
            *existing = window;
            return;
        }

        self.windows.push(window);
    }

    fn raise_window(&mut self, id: WindowId) {
        if !self
            .windows
            .iter()
            .any(|window| window.id == id && window.mapped)
        {
            return;
        }

        self.stacking_order.retain(|window_id| *window_id != id);
        self.stacking_order.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::{DisplayEvent, DisplayOutput, DisplayState, Rect, WindowId, WindowInfo};

    #[test]
    fn tracks_outputs_and_primary_display() {
        let mut state = DisplayState::new();

        state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
            "HDMI-1",
            Rect::new(0, 0, 1920, 1080),
            true,
        )));
        state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
            "DP-1",
            Rect::new(1920, 0, 1280, 720),
            true,
        )));
        state.apply(DisplayEvent::OutputDisconnected {
            name: "HDMI-1".to_string(),
        });

        assert_eq!(state.outputs().len(), 2);
        assert!(!state.outputs()[0].connected);
        assert!(!state.outputs()[0].primary);
        assert!(state.outputs()[1].primary);
    }

    #[test]
    fn tracks_window_stacking_focus_and_unmap() {
        let mut state = DisplayState::new();
        let first = WindowId(0x20);
        let second = WindowId(0x30);

        state.apply(DisplayEvent::WindowMapped(WindowInfo::mapped(
            first,
            Rect::new(0, 0, 800, 600),
        )));
        state.apply(DisplayEvent::WindowMapped(WindowInfo::mapped(
            second,
            Rect::new(20, 20, 1024, 768),
        )));
        state.apply(DisplayEvent::WindowRaised(first));
        state.apply(DisplayEvent::FocusChanged(Some(first)));

        assert_eq!(state.stacking_order(), &[second, first]);
        assert_eq!(state.top_window(), Some(first));
        assert_eq!(state.focused_window(), Some(first));

        state.apply(DisplayEvent::WindowUnmapped(first));

        assert_eq!(state.stacking_order(), &[second]);
        assert_eq!(state.top_window(), Some(second));
        assert_eq!(state.focused_window(), None);
    }
}
