use std::io;

use x11rb::{
    connection::Connection,
    protocol::{
        randr::{self, ConnectionExt as RandrConnectionExt, GetScreenResourcesCurrentReply},
        xproto::{
            ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoConnection,
            EventMask, MapState, StackMode,
        },
        Event,
    },
    rust_connection::RustConnection,
};

use crate::{DisplayBackend, DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};

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

    fn root_window(&self) -> u32 {
        self.connection.setup().roots[self.screen_index].root
    }

    fn snapshot_events(&self) -> io::Result<Vec<DisplayEvent>> {
        let mut events = vec![DisplayEvent::Reset];
        events.extend(self.output_events()?);
        events.extend(self.window_events()?);
        Ok(events)
    }

    fn subscribe_events(&self) -> io::Result<()> {
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

    fn output_events(&self) -> io::Result<Vec<DisplayEvent>> {
        let root = self.root_window();
        let resources = self
            .connection
            .randr_get_screen_resources_current(root)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        resources
            .outputs
            .iter()
            .map(|output| self.output_event(&resources, *output))
            .collect()
    }

    fn output_event(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        output: randr::Output,
    ) -> io::Result<DisplayEvent> {
        let info = self
            .connection
            .randr_get_output_info(output, resources.config_timestamp)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let name = String::from_utf8_lossy(&info.name).into_owned();

        if info.connection != randr::Connection::CONNECTED || info.crtc == 0 {
            return Ok(DisplayEvent::OutputDisconnected { name });
        }

        let crtc = self
            .connection
            .randr_get_crtc_info(info.crtc, resources.config_timestamp)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let geometry = Rect::new(
            i32::from(crtc.x),
            i32::from(crtc.y),
            u32::from(crtc.width),
            u32::from(crtc.height),
        );

        Ok(DisplayEvent::OutputConnected(DisplayOutput::connected(
            name, geometry, false,
        )))
    }

    fn window_events(&self) -> io::Result<Vec<DisplayEvent>> {
        let root = self.root_window();
        let tree = self
            .connection
            .query_tree(root)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let mut events = Vec::new();

        for window in tree.children {
            let attributes = match self
                .connection
                .get_window_attributes(window)
                .map_err(to_io_error)?
                .reply()
            {
                Ok(attributes) => attributes,
                Err(_) => continue,
            };
            if attributes.map_state != MapState::VIEWABLE {
                continue;
            }

            let geometry = match self
                .connection
                .get_geometry(window)
                .map_err(to_io_error)?
                .reply()
            {
                Ok(geometry) => geometry,
                Err(_) => continue,
            };
            let mut window_info = WindowInfo::mapped(
                WindowId(u64::from(window)),
                Rect::new(
                    i32::from(geometry.x),
                    i32::from(geometry.y),
                    u32::from(geometry.width),
                    u32::from(geometry.height),
                ),
            );
            window_info.title = self.window_title(window)?;

            events.push(DisplayEvent::WindowMapped(window_info));
        }

        if let Ok(focus) = self
            .connection
            .get_input_focus()
            .map_err(to_io_error)?
            .reply()
        {
            events.push(DisplayEvent::FocusChanged(Some(WindowId(u64::from(
                focus.focus,
            )))));
        }

        Ok(events)
    }

    fn window_title(&self, window: u32) -> io::Result<Option<String>> {
        let atom_name = self.intern_atom("_NET_WM_NAME")?;
        let atom_utf8 = self.intern_atom("UTF8_STRING")?;
        let property = self
            .connection
            .get_property(false, window, atom_name, atom_utf8, 0, 1024)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        if property.value.is_empty() {
            return Ok(None);
        }

        Ok(Some(
            String::from_utf8_lossy(&property.value)
                .trim_end_matches('\0')
                .to_string(),
        ))
    }

    fn intern_atom(&self, name: &str) -> io::Result<u32> {
        Ok(self
            .connection
            .intern_atom(false, name.as_bytes())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?
            .atom)
    }

    pub fn raise_window(&self, id: WindowId) -> io::Result<()> {
        self.stack_window(id, StackMode::ABOVE)
    }

    pub fn lower_window(&self, id: WindowId) -> io::Result<()> {
        self.stack_window(id, StackMode::BELOW)
    }

    fn stack_window(&self, id: WindowId, stack_mode: StackMode) -> io::Result<()> {
        let window = u32::try_from(id.0).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("window id {id} does not fit in an X11 window id"),
            )
        })?;
        let changes = ConfigureWindowAux::new().stack_mode(stack_mode);

        self.connection
            .configure_window(window, &changes)
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;
        self.connection.flush().map_err(to_io_error)
    }

    fn wait_for_relevant_event(&self) -> io::Result<()> {
        loop {
            let event = self.connection.wait_for_event().map_err(to_io_error)?;
            if is_relevant_event(&event) {
                return Ok(());
            }
        }
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

fn is_relevant_event(event: &Event) -> bool {
    matches!(
        event,
        Event::RandrNotify(_)
            | Event::RandrScreenChangeNotify(_)
            | Event::ConfigureNotify(_)
            | Event::CreateNotify(_)
            | Event::DestroyNotify(_)
            | Event::MapNotify(_)
            | Event::UnmapNotify(_)
            | Event::ReparentNotify(_)
            | Event::PropertyNotify(_)
            | Event::FocusIn(_)
            | Event::FocusOut(_)
    )
}

fn to_io_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}
