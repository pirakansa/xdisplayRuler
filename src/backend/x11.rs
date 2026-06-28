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

use crate::{
    DisplayBackend, DisplayEvent, DisplayOutput, Rect, WindowGeometryChange, WindowId, WindowInfo,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct X11Snapshot {
    outputs: Vec<X11OutputSnapshot>,
    windows: Vec<X11WindowSnapshot>,
    focused_window: Option<WindowId>,
}

impl X11Snapshot {
    fn into_events(self) -> Vec<DisplayEvent> {
        let mut events = vec![DisplayEvent::Reset];

        events.extend(self.outputs.into_iter().map(X11OutputSnapshot::into_event));
        events.extend(
            self.windows
                .into_iter()
                .map(|window| DisplayEvent::WindowMapped(window.into_window_info())),
        );
        events.push(DisplayEvent::FocusChanged(self.focused_window));

        events
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X11OutputSnapshot {
    name: String,
    geometry: Option<Rect>,
    primary: bool,
}

impl X11OutputSnapshot {
    fn connected(name: impl Into<String>, geometry: Rect, primary: bool) -> Self {
        Self {
            name: name.into(),
            geometry: Some(geometry),
            primary,
        }
    }

    fn disconnected(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            geometry: None,
            primary: false,
        }
    }

    fn into_event(self) -> DisplayEvent {
        match self.geometry {
            Some(geometry) => DisplayEvent::OutputConnected(DisplayOutput::connected(
                self.name,
                geometry,
                self.primary,
            )),
            None => DisplayEvent::OutputDisconnected { name: self.name },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X11WindowSnapshot {
    id: WindowId,
    title: Option<String>,
    geometry: Rect,
}

impl X11WindowSnapshot {
    fn into_window_info(self) -> WindowInfo {
        let mut window = WindowInfo::mapped(self.id, self.geometry);
        window.title = self.title;
        window
    }
}

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
        Ok(self.snapshot()?.into_events())
    }

    fn snapshot(&self) -> io::Result<X11Snapshot> {
        Ok(X11Snapshot {
            outputs: self.output_snapshots()?,
            windows: self.window_snapshots()?,
            focused_window: self.focused_window()?,
        })
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

    fn output_snapshots(&self) -> io::Result<Vec<X11OutputSnapshot>> {
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
            .map(|output| self.output_snapshot(&resources, *output))
            .collect()
    }

    fn output_snapshot(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        output: randr::Output,
    ) -> io::Result<X11OutputSnapshot> {
        let info = self
            .connection
            .randr_get_output_info(output, resources.config_timestamp)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let name = String::from_utf8_lossy(&info.name).into_owned();

        if info.connection != randr::Connection::CONNECTED || info.crtc == 0 {
            return Ok(X11OutputSnapshot::disconnected(name));
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

        Ok(X11OutputSnapshot::connected(name, geometry, false))
    }

    fn window_snapshots(&self) -> io::Result<Vec<X11WindowSnapshot>> {
        let root = self.root_window();
        let tree = self
            .connection
            .query_tree(root)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let mut windows = Vec::new();

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
            windows.push(X11WindowSnapshot {
                id: WindowId(u64::from(window)),
                title: self.window_title(window)?,
                geometry: Rect::new(
                    i32::from(geometry.x),
                    i32::from(geometry.y),
                    u32::from(geometry.width),
                    u32::from(geometry.height),
                ),
            });
        }

        Ok(windows)
    }

    fn focused_window(&self) -> io::Result<Option<WindowId>> {
        if let Ok(focus) = self
            .connection
            .get_input_focus()
            .map_err(to_io_error)?
            .reply()
        {
            return Ok(Some(WindowId(u64::from(focus.focus))));
        }

        Ok(None)
    }

    fn window_title(&self, window: u32) -> io::Result<Option<String>> {
        if let Some(title) = self.window_text_property(window, "_NET_WM_NAME", "UTF8_STRING")? {
            return Ok(Some(title));
        }

        self.window_text_property(window, "WM_NAME", "STRING")
    }

    fn window_text_property(
        &self,
        window: u32,
        property_name: &str,
        property_type: &str,
    ) -> io::Result<Option<String>> {
        let property_atom = self.intern_atom(property_name)?;
        let type_atom = self.intern_atom(property_type)?;
        let property = self
            .connection
            .get_property(false, window, property_atom, type_atom, 0, 1024)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        Ok(text_property_value(&property.value))
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
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;
        self.connection.flush().map_err(to_io_error)
    }

    fn stack_window(&self, id: WindowId, stack_mode: StackMode) -> io::Result<()> {
        let window = x11_window_id(id)?;
        let changes = ConfigureWindowAux::new().stack_mode(stack_mode);

        self.connection
            .configure_window(window, &changes)
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;
        self.connection.flush().map_err(to_io_error)
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
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;
        self.connection.flush().map_err(to_io_error)
    }
}

fn text_property_value(value: &[u8]) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    let text = String::from_utf8_lossy(value)
        .trim_end_matches('\0')
        .to_string();

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn x11_window_id(id: WindowId) -> io::Result<u32> {
    u32::try_from(id.0).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("window id {id} does not fit in an X11 window id"),
        )
    })
}

impl X11Backend {
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

#[cfg(test)]
mod tests {
    use super::{
        text_property_value, x11_window_id, X11OutputSnapshot, X11Snapshot, X11WindowSnapshot,
    };
    use crate::{DisplayEvent, DisplayOutput, Rect, WindowId, WindowInfo};

    #[test]
    fn converts_snapshot_to_reset_and_current_state_events() {
        let snapshot = X11Snapshot {
            outputs: vec![
                X11OutputSnapshot::connected("HDMI-1", Rect::new(0, 0, 1920, 1080), true),
                X11OutputSnapshot::disconnected("DP-1"),
            ],
            windows: vec![
                X11WindowSnapshot {
                    id: WindowId(0x10),
                    title: Some("first".to_string()),
                    geometry: Rect::new(0, 0, 800, 600),
                },
                X11WindowSnapshot {
                    id: WindowId(0x20),
                    title: None,
                    geometry: Rect::new(800, 0, 640, 480),
                },
            ],
            focused_window: Some(WindowId(0x20)),
        };

        let events = snapshot.into_events();

        assert_eq!(events[0], DisplayEvent::Reset);
        assert_eq!(
            events[1],
            DisplayEvent::OutputConnected(DisplayOutput::connected(
                "HDMI-1",
                Rect::new(0, 0, 1920, 1080),
                true,
            ))
        );
        assert_eq!(
            events[2],
            DisplayEvent::OutputDisconnected {
                name: "DP-1".to_string(),
            }
        );
        assert_eq!(
            events[3],
            DisplayEvent::WindowMapped(WindowInfo {
                id: WindowId(0x10),
                title: Some("first".to_string()),
                app_id: None,
                geometry: Rect::new(0, 0, 800, 600),
                mapped: true,
            })
        );
        assert_eq!(events[5], DisplayEvent::FocusChanged(Some(WindowId(0x20))));
    }

    #[test]
    fn disconnected_outputs_ignore_primary_flag() {
        let event = X11OutputSnapshot::disconnected("HDMI-2").into_event();

        assert_eq!(
            event,
            DisplayEvent::OutputDisconnected {
                name: "HDMI-2".to_string(),
            }
        );
    }

    #[test]
    fn validates_x11_window_id_range() {
        assert_eq!(
            x11_window_id(WindowId(u64::from(u32::MAX))).expect("id should fit"),
            u32::MAX
        );
        assert!(x11_window_id(WindowId(u64::from(u32::MAX) + 1)).is_err());
    }

    #[test]
    fn normalizes_text_property_values() {
        assert_eq!(
            text_property_value(b"plain title"),
            Some("plain title".to_string())
        );
        assert_eq!(
            text_property_value(b"legacy title\0\0"),
            Some("legacy title".to_string())
        );
        assert_eq!(text_property_value(b""), None);
        assert_eq!(text_property_value(b"\0\0"), None);
    }
}
