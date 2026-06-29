use std::io;

mod control;
mod event;
mod mode;
mod output_control;
mod snapshot;
mod touch;
mod touch_control;
mod types;
mod window;

use x11rb::{
    connection::Connection,
    protocol::{
        randr::{self, ConnectionExt as RandrConnectionExt, GetScreenResourcesCurrentReply},
        xproto::{ChangeWindowAttributesAux, ConnectionExt as XprotoConnection, EventMask},
    },
    rust_connection::RustConnection,
};

use crate::{DisplayBackend, DisplayEvent};

use event::is_relevant_event;

const COORDINATE_TRANSFORMATION_MATRIX: &str = "Coordinate Transformation Matrix";
const FLOAT_ATOM: &str = "FLOAT";

#[derive(Debug)]
pub struct X11Backend {
    connection: RustConnection,
    screen_index: usize,
    initial_snapshot_pending: bool,
}

impl X11Backend {
    pub fn connect() -> io::Result<Self> {
        let (connection, screen_index) =
            x11rb::connect(None).map_err(|error| io::Error::other(error.to_string()))?;

        let randr_extension = connection
            .query_extension(randr::X11_EXTENSION_NAME.as_bytes())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        if !randr_extension.present {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Xorg server does not provide the RANDR extension",
            ));
        }

        let backend = Self {
            connection,
            screen_index,
            initial_snapshot_pending: true,
        };
        backend.subscribe_events()?;
        Ok(backend)
    }

    pub(super) fn root_window(&self) -> u32 {
        self.connection.setup().roots[self.screen_index].root
    }

    pub(super) fn subscribe_events(&self) -> io::Result<()> {
        let root = self.root_window();
        let randr_mask = randr::NotifyMask::SCREEN_CHANGE
            | randr::NotifyMask::CRTC_CHANGE
            | randr::NotifyMask::OUTPUT_CHANGE
            | randr::NotifyMask::RESOURCE_CHANGE;
        self.connection
            .randr_select_input(root, randr_mask)
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;

        let root_event_mask = EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::STRUCTURE_NOTIFY
            | EventMask::PROPERTY_CHANGE
            | EventMask::FOCUS_CHANGE;
        let attributes = ChangeWindowAttributesAux::new().event_mask(root_event_mask);
        self.connection
            .change_window_attributes(root, &attributes)
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;
        self.connection.flush().map_err(to_io_error)
    }

    pub(super) fn screen_resources(&self) -> io::Result<GetScreenResourcesCurrentReply> {
        self.connection
            .randr_get_screen_resources_current(self.root_window())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)
    }

    pub(super) fn output_by_name(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        output_name: &str,
    ) -> io::Result<(randr::Output, randr::GetOutputInfoReply)> {
        for output in &resources.outputs {
            let info = self
                .connection
                .randr_get_output_info(*output, resources.config_timestamp)
                .map_err(to_io_error)?
                .reply()
                .map_err(to_io_error)?;
            let name = String::from_utf8_lossy(&info.name);

            if name == output_name {
                return Ok((*output, info));
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("output not found: {output_name}"),
        ))
    }

    pub(super) fn intern_atom(&self, name: &str) -> io::Result<u32> {
        Ok(self
            .connection
            .intern_atom(false, name.as_bytes())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?
            .atom)
    }
}

impl DisplayBackend for X11Backend {
    fn name(&self) -> &'static str {
        "x11"
    }

    fn poll_events(&mut self) -> io::Result<Vec<DisplayEvent>> {
        if !self.initial_snapshot_pending {
            self.wait_for_relevant_event()?;
        }

        self.initial_snapshot_pending = false;
        self.snapshot_events()
    }
}

pub(super) fn to_io_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}
