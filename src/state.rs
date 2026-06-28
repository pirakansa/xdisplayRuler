use crate::models::{DisplayEvent, DisplayOutput, WindowId, WindowInfo};

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
            DisplayEvent::Reset => *self = Self::new(),
            DisplayEvent::OutputConnected(output) => self.upsert_output(output),
            DisplayEvent::OutputDisconnected { name } => self.mark_output_disconnected(&name),
            DisplayEvent::OutputGeometryChanged { name, geometry } => {
                if let Some(output) = self.find_output_mut(&name) {
                    output.geometry = geometry;
                    output.connected = true;
                }
            }
            DisplayEvent::WindowMapped(window) => {
                let id = window.id;
                self.upsert_window(window);
                self.raise_window(id);
            }
            DisplayEvent::WindowUnmapped(id) => self.mark_window_unmapped(id),
            DisplayEvent::WindowConfigured { id, geometry } => {
                if let Some(window) = self.find_window_mut(id) {
                    window.geometry = geometry;
                }
            }
            DisplayEvent::WindowRaised(id) => self.raise_window(id),
            DisplayEvent::FocusChanged(id) => {
                self.focused_window = id.filter(|id| self.is_mapped_window(*id));
            }
        }
    }

    pub fn status_report(&self) -> String {
        self.status_report_for_backend("in-memory")
    }

    pub fn status_report_for_backend(&self, backend_name: &str) -> String {
        let mut report = format!("xdisplay-ruler\nbackend: {backend_name}\n");
        report.push_str(&format!("outputs: {}\n", self.outputs.len()));
        report.push_str(&self.output_report());
        report.push_str(&format!("windows: {}\n", self.windows.len()));
        report.push_str(&self.window_report());
        report.push_str(&format!("focused: {}\n", self.focused_window_label()));
        report.push_str(&format!("top: {}\n", self.top_window_label()));
        report
    }

    fn output_report(&self) -> String {
        self.outputs
            .iter()
            .map(|output| {
                let primary = if output.primary { " primary" } else { "" };
                let status = if output.connected {
                    "connected"
                } else {
                    "disconnected"
                };

                format!(
                    "- {}: {} {}{}\n",
                    output.name, status, output.geometry, primary
                )
            })
            .collect()
    }

    fn window_report(&self) -> String {
        self.windows
            .iter()
            .map(|window| {
                let mapped = if window.mapped { "mapped" } else { "unmapped" };
                let title = window
                    .title
                    .as_deref()
                    .map(window_title_report)
                    .unwrap_or_default();
                format!("- {}: {} {}{}\n", window.id, mapped, window.geometry, title)
            })
            .collect()
    }

    fn focused_window_label(&self) -> String {
        self.focused_window
            .map_or_else(|| "none".to_string(), |id| id.to_string())
    }

    fn top_window_label(&self) -> String {
        self.top_window()
            .map_or_else(|| "none".to_string(), |id| id.to_string())
    }

    fn upsert_output(&mut self, output: DisplayOutput) {
        if output.primary {
            self.clear_primary_output();
        }

        if let Some(existing) = self.find_output_mut(&output.name) {
            *existing = output;
            return;
        }

        self.outputs.push(output);
    }

    fn mark_output_disconnected(&mut self, name: &str) {
        if let Some(output) = self.find_output_mut(name) {
            output.connected = false;
            output.primary = false;
        }
    }

    fn clear_primary_output(&mut self) {
        for output in &mut self.outputs {
            output.primary = false;
        }
    }

    fn find_output_mut(&mut self, name: &str) -> Option<&mut DisplayOutput> {
        self.outputs.iter_mut().find(|output| output.name == name)
    }

    fn upsert_window(&mut self, window: WindowInfo) {
        if let Some(existing) = self.find_window_mut(window.id) {
            *existing = window;
            return;
        }

        self.windows.push(window);
    }

    fn mark_window_unmapped(&mut self, id: WindowId) {
        if let Some(window) = self.find_window_mut(id) {
            window.mapped = false;
        }

        self.stacking_order.retain(|window_id| *window_id != id);

        if self.focused_window == Some(id) {
            self.focused_window = None;
        }
    }

    fn find_window_mut(&mut self, id: WindowId) -> Option<&mut WindowInfo> {
        self.windows.iter_mut().find(|window| window.id == id)
    }

    fn raise_window(&mut self, id: WindowId) {
        if !self.is_mapped_window(id) {
            return;
        }

        self.stacking_order.retain(|window_id| *window_id != id);
        self.stacking_order.push(id);
    }

    fn is_mapped_window(&self, id: WindowId) -> bool {
        self.windows
            .iter()
            .any(|window| window.id == id && window.mapped)
    }
}

fn window_title_report(title: &str) -> String {
    if title.is_empty() {
        return String::new();
    }

    format!(" title=\"{}\"", escape_report_value(title))
}

fn escape_report_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use crate::{DisplayEvent, DisplayOutput, DisplayState, Rect, WindowId, WindowInfo};

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

    #[test]
    fn status_report_includes_escaped_window_title_when_present() {
        let mut state = DisplayState::new();
        let mut window = WindowInfo::mapped(WindowId(0x20), Rect::new(0, 0, 800, 600));
        window.title = Some("hello \"display\"\n".to_string());

        state.apply(DisplayEvent::WindowMapped(window));

        assert!(state
            .status_report()
            .contains("- 0x20: mapped 800x600+0+0 title=\"hello \\\"display\\\"\\n\""));
    }

    #[test]
    fn reset_clears_outputs_windows_stacking_and_focus() {
        let mut state = DisplayState::new();

        state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
            "HDMI-1",
            Rect::new(0, 0, 1920, 1080),
            true,
        )));
        state.apply(DisplayEvent::WindowMapped(WindowInfo::mapped(
            WindowId(0x40),
            Rect::new(0, 0, 800, 600),
        )));
        state.apply(DisplayEvent::FocusChanged(Some(WindowId(0x40))));

        state.apply(DisplayEvent::Reset);

        assert!(state.outputs().is_empty());
        assert!(state.windows().is_empty());
        assert!(state.stacking_order().is_empty());
        assert_eq!(state.focused_window(), None);
    }
}
