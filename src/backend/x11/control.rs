use std::io;

use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConfigureWindowAux, ConnectionExt as XprotoConnection, InputFocus, StackMode,
    },
    CURRENT_TIME,
};

use crate::{Rect, WindowGeometryChange, WindowId};

use super::{window::x11_window_id, X11Backend};

impl X11Backend {
    pub fn raise_window(&self, id: WindowId) -> io::Result<()> {
        self.stack_window(id, StackMode::ABOVE)
    }

    pub fn lower_window(&self, id: WindowId) -> io::Result<()> {
        self.stack_window(id, StackMode::BELOW)
    }

    pub fn stack_window_above(&self, id: WindowId, sibling: WindowId) -> io::Result<()> {
        let window = x11_window_id(id)?;
        let sibling = x11_window_id(sibling)?;
        let changes = ConfigureWindowAux::new()
            .sibling(sibling)
            .stack_mode(StackMode::ABOVE);

        self.connection
            .configure_window(window, &changes)
            .map_err(super::to_io_error)?
            .check()
            .map_err(super::to_io_error)?;
        self.connection.flush().map_err(super::to_io_error)
    }

    pub fn activate_window(&self, id: WindowId) -> io::Result<()> {
        let window = x11_window_id(id)?;

        self.connection
            .set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)
            .map_err(super::to_io_error)?
            .check()
            .map_err(super::to_io_error)?;
        self.connection.flush().map_err(super::to_io_error)
    }

    pub fn place_window_fullscreen(&self, id: WindowId, output_name: &str) -> io::Result<()> {
        let output = self
            .output_snapshots()?
            .into_iter()
            .find(|output| output.name == output_name)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("output not found: {output_name}"),
                )
            })?;
        let geometry = output.geometry.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("output is disconnected: {output_name}"),
            )
        })?;

        self.configure_window_geometry(id, &geometry)?;
        self.raise_window(id)
    }

    pub fn configure_window(&self, id: WindowId, change: &WindowGeometryChange) -> io::Result<()> {
        if change.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "at least one of --x, --y, --width, or --height is required",
            ));
        }

        let window = x11_window_id(id)?;
        let mut changes = ConfigureWindowAux::new();

        if let Some(x) = change.x {
            changes = changes.x(x);
        }
        if let Some(y) = change.y {
            changes = changes.y(y);
        }
        if let Some(width) = change.width {
            changes = changes.width(width);
        }
        if let Some(height) = change.height {
            changes = changes.height(height);
        }

        self.connection
            .configure_window(window, &changes)
            .map_err(super::to_io_error)?
            .check()
            .map_err(super::to_io_error)?;
        self.connection.flush().map_err(super::to_io_error)
    }

    fn stack_window(&self, id: WindowId, stack_mode: StackMode) -> io::Result<()> {
        let window = x11_window_id(id)?;
        let changes = ConfigureWindowAux::new().stack_mode(stack_mode);

        self.connection
            .configure_window(window, &changes)
            .map_err(super::to_io_error)?
            .check()
            .map_err(super::to_io_error)?;
        self.connection.flush().map_err(super::to_io_error)
    }

    fn configure_window_geometry(&self, id: WindowId, geometry: &Rect) -> io::Result<()> {
        let window = x11_window_id(id)?;
        let changes = ConfigureWindowAux::new()
            .x(geometry.x)
            .y(geometry.y)
            .width(geometry.width)
            .height(geometry.height);

        self.connection
            .configure_window(window, &changes)
            .map_err(super::to_io_error)?
            .check()
            .map_err(super::to_io_error)?;
        self.connection.flush().map_err(super::to_io_error)
    }
}
