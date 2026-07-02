use std::io;

use crate::{ConfiguredBackend, DisplayEvent, WindowGeometryChange, WindowId};

pub(crate) trait WindowLayoutBackend {
    fn snapshot_events(&mut self) -> io::Result<Vec<DisplayEvent>>;

    fn configure_window(&self, id: WindowId, change: &WindowGeometryChange) -> io::Result<()>;

    fn raise_window(&self, id: WindowId) -> io::Result<()>;

    fn activate_window(&self, id: WindowId) -> io::Result<()>;

    fn stack_window_above(&self, id: WindowId, sibling: WindowId) -> io::Result<()>;
}

impl WindowLayoutBackend for ConfiguredBackend {
    fn snapshot_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        Self::snapshot_events(self)
    }

    fn configure_window(&self, id: WindowId, change: &WindowGeometryChange) -> io::Result<()> {
        Self::configure_window(self, id, change)
    }

    fn raise_window(&self, id: WindowId) -> io::Result<()> {
        Self::raise_window(self, id)
    }

    fn activate_window(&self, id: WindowId) -> io::Result<()> {
        Self::activate_window(self, id)
    }

    fn stack_window_above(&self, id: WindowId, sibling: WindowId) -> io::Result<()> {
        Self::stack_window_above(self, id, sibling)
    }
}
