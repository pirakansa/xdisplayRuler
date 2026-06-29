mod report;

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

#[cfg(test)]
mod tests;
