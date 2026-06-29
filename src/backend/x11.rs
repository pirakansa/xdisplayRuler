use std::io;

use x11rb::{
    connection::Connection,
    protocol::{
        randr::{self, ConnectionExt as RandrConnectionExt, GetScreenResourcesCurrentReply},
        xinput::{
            ConnectionExt as XinputConnectionExt, Device, DeviceClassData, XIChangePropertyAux,
            XIDeviceInfo,
        },
        xproto::{
            ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoConnection,
            EventMask, MapState, PropMode, StackMode,
        },
        Event,
    },
    rust_connection::RustConnection,
};

use crate::{
    DisplayBackend, DisplayEvent, DisplayOutput, OutputMode, OutputModeChange, OutputModeSelection,
    OutputRotation, Rect, WindowGeometryChange, WindowId, WindowInfo,
};

const COORDINATE_TRANSFORMATION_MATRIX: &str = "Coordinate Transformation Matrix";
const FLOAT_ATOM: &str = "FLOAT";

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
    class_name: Option<String>,
    instance_name: Option<String>,
    geometry: Rect,
}

impl X11WindowSnapshot {
    fn into_window_info(self) -> WindowInfo {
        let mut window = WindowInfo::mapped(self.id, self.geometry);
        window.title = self.title;
        window.class_name = self.class_name;
        window.instance_name = self.instance_name;
        window
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X11WindowClass {
    instance_name: String,
    class_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct X11ModeInfo {
    id: randr::Mode,
    name: String,
    width: u16,
    height: u16,
    refresh_millihertz: Option<u32>,
}

impl X11ModeInfo {
    fn public_mode(self, preferred: bool, current: bool) -> OutputMode {
        OutputMode {
            name: self.name,
            width: self.width,
            height: self.height,
            refresh_millihertz: self.refresh_millihertz,
            preferred,
            current,
        }
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

    pub(crate) fn snapshot_events(&self) -> io::Result<Vec<DisplayEvent>> {
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

    fn screen_resources(&self) -> io::Result<GetScreenResourcesCurrentReply> {
        self.connection
            .randr_get_screen_resources_current(self.root_window())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)
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

    fn output_by_name(
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
            let class = self.window_class(window)?;
            windows.push(X11WindowSnapshot {
                id: WindowId(u64::from(window)),
                title: self.window_title(window)?,
                class_name: class.as_ref().map(|class| class.class_name.clone()),
                instance_name: class.map(|class| class.instance_name),
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

    pub fn windows(&self) -> io::Result<Vec<WindowInfo>> {
        Ok(self
            .window_snapshots()?
            .into_iter()
            .map(X11WindowSnapshot::into_window_info)
            .collect())
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

    fn window_class(&self, window: u32) -> io::Result<Option<X11WindowClass>> {
        let Some(value) = self.window_bytes_property(window, "WM_CLASS", "STRING")? else {
            return Ok(None);
        };

        Ok(window_class_value(&value))
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

    fn window_bytes_property(
        &self,
        window: u32,
        property_name: &str,
        property_type: &str,
    ) -> io::Result<Option<Vec<u8>>> {
        let property_atom = self.intern_atom(property_name)?;
        let type_atom = self.intern_atom(property_type)?;
        let property = self
            .connection
            .get_property(false, window, property_atom, type_atom, 0, 1024)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        if property.value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(property.value))
        }
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

    pub fn output_modes(&self, output_name: &str) -> io::Result<Vec<OutputMode>> {
        let resources = self.screen_resources()?;
        let (_, output) = self.output_by_name(&resources, output_name)?;
        ensure_connected_output(output_name, &output)?;
        let current_mode = self.current_output_mode(&resources, &output)?;
        let mode_infos = mode_infos(&resources);

        output
            .modes
            .iter()
            .enumerate()
            .map(|(index, mode)| {
                let info = mode_infos
                    .iter()
                    .find(|info| info.id == *mode)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("mode id {mode} was not present in screen resources"),
                        )
                    })?
                    .clone();
                Ok(info.public_mode(
                    index < usize::from(output.num_preferred),
                    current_mode == *mode,
                ))
            })
            .collect()
    }

    pub fn set_output_mode(
        &self,
        output_name: &str,
        selection: &OutputModeSelection,
    ) -> io::Result<OutputModeChange> {
        let resources = self.screen_resources()?;
        let (output_id, output) = self.output_by_name(&resources, output_name)?;
        ensure_connected_output(output_name, &output)?;

        if output.crtc == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("output has no active CRTC: {output_name}"),
            ));
        }

        let crtc = self
            .connection
            .randr_get_crtc_info(output.crtc, resources.config_timestamp)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let selected_mode = self.select_output_mode(&resources, &output, &crtc, selection)?;
        let selected_rotation = selected_output_rotation(crtc.rotation, selection.rotation);
        let outputs = if crtc.outputs.is_empty() {
            vec![output_id]
        } else {
            crtc.outputs
        };
        let reply = self
            .connection
            .randr_set_crtc_config(
                output.crtc,
                0,
                resources.config_timestamp,
                crtc.x,
                crtc.y,
                selected_mode.id,
                selected_rotation,
                &outputs,
            )
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        if reply.status != randr::SetConfig::SUCCESS {
            return Err(io::Error::other(format!(
                "RandR set CRTC config failed: {}",
                set_config_status_label(reply.status)
            )));
        }

        let mut change = OutputModeChange::default();
        if let Err(error) = self.remap_touch_devices_to_output(output_name, selected_rotation) {
            change.warnings.push(format!(
                "output mode changed, but touch remapping failed: {error}"
            ));
        }

        self.connection.flush().map_err(to_io_error)?;
        Ok(change)
    }

    fn remap_touch_devices_to_output(
        &self,
        output_name: &str,
        rotation: randr::Rotation,
    ) -> io::Result<()> {
        let root_geometry = self
            .connection
            .get_geometry(self.root_window())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;
        let root = Rect::new(
            0,
            0,
            u32::from(root_geometry.width),
            u32::from(root_geometry.height),
        );
        let output = self
            .output_snapshots()?
            .into_iter()
            .find(|output| output.name == output_name)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("output not found after mode switch: {output_name}"),
                )
            })?;
        let output = output.geometry.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("output is disconnected after mode switch: {output_name}"),
            )
        })?;
        let matrix = coordinate_transformation_matrix(&root, &output, rotation)?;

        self.apply_touch_coordinate_transformation(matrix)
    }

    fn apply_touch_coordinate_transformation(&self, matrix: [f32; 9]) -> io::Result<()> {
        let property_atom = self.intern_atom(COORDINATE_TRANSFORMATION_MATRIX)?;
        let float_atom = self.intern_atom(FLOAT_ATOM)?;
        let matrix_bits = matrix.into_iter().map(f32::to_bits).collect::<Vec<u32>>();
        let property = XIChangePropertyAux::Data32(matrix_bits);
        let devices = self
            .connection
            .xinput_xi_query_device(Device::ALL)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        for device in devices.infos.iter().filter(|device| touch_device(device)) {
            self.connection
                .xinput_xi_change_property(
                    device.deviceid,
                    PropMode::REPLACE,
                    property_atom,
                    float_atom,
                    9,
                    &property,
                )
                .map_err(to_io_error)?
                .check()
                .map_err(to_io_error)?;
        }

        Ok(())
    }

    fn current_output_mode(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        output: &randr::GetOutputInfoReply,
    ) -> io::Result<randr::Mode> {
        if output.crtc == 0 {
            return Ok(0);
        }

        Ok(self
            .connection
            .randr_get_crtc_info(output.crtc, resources.config_timestamp)
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?
            .mode)
    }

    fn select_output_mode(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        output: &randr::GetOutputInfoReply,
        crtc: &randr::GetCrtcInfoReply,
        selection: &OutputModeSelection,
    ) -> io::Result<X11ModeInfo> {
        let (Some(width), Some(height)) = (selection.width, selection.height) else {
            if crtc.mode == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "output has no active mode to reuse",
                ));
            }

            return mode_infos(resources)
                .into_iter()
                .find(|mode| mode.id == crtc.mode)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "active mode id {} was not present in screen resources",
                            crtc.mode
                        ),
                    )
                });
        };

        let mode_infos = mode_infos(resources);
        let mut candidates = output
            .modes
            .iter()
            .filter_map(|mode| mode_infos.iter().find(|info| info.id == *mode))
            .filter(|mode| mode.width == width && mode.height == height)
            .filter(|mode| refresh_matches(mode.refresh_millihertz, selection.refresh_millihertz))
            .cloned()
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                mode_not_found_message(selection),
            ));
        }

        if let Some(target_rate) = selection.refresh_millihertz {
            candidates.sort_by_key(|mode| {
                mode.refresh_millihertz
                    .map(|rate| rate.abs_diff(target_rate))
                    .unwrap_or(u32::MAX)
            });
        }

        Ok(candidates.remove(0))
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

fn ensure_connected_output(
    output_name: &str,
    output: &randr::GetOutputInfoReply,
) -> io::Result<()> {
    if output.connection != randr::Connection::CONNECTED {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("output is disconnected: {output_name}"),
        ));
    }

    Ok(())
}

fn mode_infos(resources: &GetScreenResourcesCurrentReply) -> Vec<X11ModeInfo> {
    let mut name_offset = 0;

    resources
        .modes
        .iter()
        .map(|mode| {
            let name_len = usize::from(mode.name_len);
            let name_end = name_offset + name_len;
            let name = resources
                .names
                .get(name_offset..name_end)
                .map(|name| String::from_utf8_lossy(name).into_owned())
                .unwrap_or_default();
            name_offset = name_end;

            X11ModeInfo {
                id: mode.id,
                name,
                width: mode.width,
                height: mode.height,
                refresh_millihertz: refresh_millihertz(mode),
            }
        })
        .collect()
}

fn refresh_millihertz(mode: &randr::ModeInfo) -> Option<u32> {
    if mode.dot_clock == 0 || mode.htotal == 0 || mode.vtotal == 0 {
        return None;
    }

    let mut refresh =
        u64::from(mode.dot_clock) * 1000 / (u64::from(mode.htotal) * u64::from(mode.vtotal));

    if (mode.mode_flags & randr::ModeFlag::INTERLACE) == randr::ModeFlag::INTERLACE {
        refresh *= 2;
    }
    if (mode.mode_flags & randr::ModeFlag::DOUBLE_SCAN) == randr::ModeFlag::DOUBLE_SCAN {
        refresh /= 2;
    }

    u32::try_from(refresh).ok()
}

fn refresh_matches(actual: Option<u32>, expected: Option<u32>) -> bool {
    match (actual, expected) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual.abs_diff(expected) <= 500,
        (None, Some(_)) => false,
    }
}

fn mode_not_found_message(selection: &OutputModeSelection) -> String {
    let rate = selection
        .refresh_millihertz
        .map(|rate| format!(" at {}", format_refresh_millihertz(rate)))
        .unwrap_or_default();
    let size = match (selection.width, selection.height) {
        (Some(width), Some(height)) => format!("{width}x{height}"),
        _ => "active mode".to_string(),
    };

    format!("output mode not found: {size}{rate}")
}

fn format_refresh_millihertz(refresh_millihertz: u32) -> String {
    let hz = refresh_millihertz / 1000;
    let fraction = refresh_millihertz % 1000;

    if fraction == 0 {
        format!("{hz}Hz")
    } else {
        let mut fraction = format!("{fraction:03}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        format!("{hz}.{fraction}Hz")
    }
}

fn set_config_status_label(status: randr::SetConfig) -> &'static str {
    match status {
        randr::SetConfig::SUCCESS => "success",
        randr::SetConfig::INVALID_CONFIG_TIME => "invalid config time",
        randr::SetConfig::INVALID_TIME => "invalid time",
        randr::SetConfig::FAILED => "failed",
        _ => "unknown",
    }
}

fn selected_output_rotation(
    current: randr::Rotation,
    selected: Option<OutputRotation>,
) -> randr::Rotation {
    let basic = selected.map_or_else(|| basic_rotation(current), output_rotation_to_randr);
    let reflection = u16::from(current)
        & (u16::from(randr::Rotation::REFLECT_X) | u16::from(randr::Rotation::REFLECT_Y));

    randr::Rotation::from(u16::from(basic) | reflection)
}

fn output_rotation_to_randr(rotation: OutputRotation) -> randr::Rotation {
    match rotation {
        OutputRotation::Normal => randr::Rotation::ROTATE0,
        OutputRotation::Left => randr::Rotation::ROTATE90,
        OutputRotation::Right => randr::Rotation::ROTATE270,
        OutputRotation::Inverted => randr::Rotation::ROTATE180,
    }
}

fn basic_rotation(rotation: randr::Rotation) -> randr::Rotation {
    let basic = u16::from(rotation)
        & (u16::from(randr::Rotation::ROTATE0)
            | u16::from(randr::Rotation::ROTATE90)
            | u16::from(randr::Rotation::ROTATE180)
            | u16::from(randr::Rotation::ROTATE270));

    match randr::Rotation::from(basic) {
        randr::Rotation::ROTATE90 => randr::Rotation::ROTATE90,
        randr::Rotation::ROTATE180 => randr::Rotation::ROTATE180,
        randr::Rotation::ROTATE270 => randr::Rotation::ROTATE270,
        _ => randr::Rotation::ROTATE0,
    }
}

fn coordinate_transformation_matrix(
    root: &Rect,
    output: &Rect,
    rotation: randr::Rotation,
) -> io::Result<[f32; 9]> {
    if root.width == 0 || root.height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "root geometry must have positive width and height",
        ));
    }

    let root_width = root.width as f32;
    let root_height = root.height as f32;
    let output_width = output.width as f32 / root_width;
    let output_height = output.height as f32 / root_height;
    let output_x = (output.x - root.x) as f32 / root_width;
    let output_y = (output.y - root.y) as f32 / root_height;

    Ok(match basic_rotation(rotation) {
        randr::Rotation::ROTATE90 => [
            0.0,
            -output_width,
            output_x + output_width,
            output_height,
            0.0,
            output_y,
            0.0,
            0.0,
            1.0,
        ],
        randr::Rotation::ROTATE180 => [
            -output_width,
            0.0,
            output_x + output_width,
            0.0,
            -output_height,
            output_y + output_height,
            0.0,
            0.0,
            1.0,
        ],
        randr::Rotation::ROTATE270 => [
            0.0,
            output_width,
            output_x,
            -output_height,
            0.0,
            output_y + output_height,
            0.0,
            0.0,
            1.0,
        ],
        _ => [
            output_width,
            0.0,
            output_x,
            0.0,
            output_height,
            output_y,
            0.0,
            0.0,
            1.0,
        ],
    })
}

fn touch_device(device: &XIDeviceInfo) -> bool {
    device
        .enabled
        .then_some(())
        .and_then(|_| {
            device
                .classes
                .iter()
                .find(|class| matches!(class.data, DeviceClassData::Touch(_)))
        })
        .is_some()
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

fn window_class_value(value: &[u8]) -> Option<X11WindowClass> {
    let mut parts = value
        .split(|byte| *byte == b'\0')
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned());
    let instance_name = parts.next()?;
    let class_name = parts.next()?;

    Some(X11WindowClass {
        instance_name,
        class_name,
    })
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
        coordinate_transformation_matrix, mode_infos, output_rotation_to_randr, refresh_millihertz,
        selected_output_rotation, text_property_value, touch_device, window_class_value,
        x11_window_id, X11OutputSnapshot, X11Snapshot, X11WindowSnapshot,
    };
    use crate::{DisplayEvent, DisplayOutput, OutputRotation, Rect, WindowId, WindowInfo};
    use x11rb::protocol::randr::{GetScreenResourcesCurrentReply, ModeFlag, ModeInfo, Rotation};
    use x11rb::protocol::xinput::{
        DeviceClass, DeviceClassData, DeviceClassDataKey, DeviceClassDataTouch, DeviceId,
        DeviceType, TouchMode, XIDeviceInfo,
    };

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
                    class_name: Some("Code".to_string()),
                    instance_name: Some("code".to_string()),
                    geometry: Rect::new(0, 0, 800, 600),
                },
                X11WindowSnapshot {
                    id: WindowId(0x20),
                    title: None,
                    class_name: None,
                    instance_name: None,
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
                class_name: Some("Code".to_string()),
                instance_name: Some("code".to_string()),
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

    #[test]
    fn parses_window_class_values() {
        let class = window_class_value(b"code\0Code\0").expect("class should parse");

        assert_eq!(class.instance_name, "code");
        assert_eq!(class.class_name, "Code");
        assert_eq!(window_class_value(b"code\0"), None);
        assert_eq!(window_class_value(b""), None);
    }

    #[test]
    fn extracts_mode_names_and_refresh_rates() {
        let resources = GetScreenResourcesCurrentReply {
            sequence: 0,
            length: 0,
            timestamp: 0,
            config_timestamp: 0,
            crtcs: Vec::new(),
            outputs: Vec::new(),
            modes: vec![
                test_mode(TestMode {
                    id: 1,
                    width: 1920,
                    height: 1080,
                    dot_clock: 148_500_000,
                    htotal: 2200,
                    vtotal: 1125,
                    mode_flags: ModeFlag::default(),
                    name_len: 9,
                }),
                test_mode(TestMode {
                    id: 2,
                    width: 1280,
                    height: 720,
                    dot_clock: 74_250_000,
                    htotal: 1650,
                    vtotal: 750,
                    mode_flags: ModeFlag::INTERLACE,
                    name_len: 8,
                }),
            ],
            names: b"1920x10801280x720".to_vec(),
        };

        let modes = mode_infos(&resources);

        assert_eq!(modes[0].name, "1920x1080");
        assert_eq!(modes[0].refresh_millihertz, Some(60_000));
        assert_eq!(modes[1].name, "1280x720");
        assert_eq!(modes[1].refresh_millihertz, Some(120_000));
    }

    #[test]
    fn calculates_double_scan_refresh_rate() {
        let mode = test_mode(TestMode {
            id: 1,
            width: 320,
            height: 240,
            dot_clock: 25_175_000,
            htotal: 800,
            vtotal: 525,
            mode_flags: ModeFlag::DOUBLE_SCAN,
            name_len: 7,
        });

        assert_eq!(refresh_millihertz(&mode), Some(29_970));
    }

    #[test]
    fn calculates_coordinate_transformation_matrix_for_output_relative_to_root() {
        let root = Rect::new(0, 0, 3840, 1080);
        let output = Rect::new(1920, 0, 1920, 1080);

        let matrix = coordinate_transformation_matrix(&root, &output, Rotation::ROTATE0)
            .expect("matrix should be calculated");

        assert_eq!(matrix, [0.5, 0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn calculates_coordinate_transformation_matrix_for_rotated_output() {
        let root = Rect::new(0, 0, 3840, 2160);
        let output = Rect::new(1920, 0, 1080, 1920);

        assert_eq!(
            coordinate_transformation_matrix(&root, &output, Rotation::ROTATE90)
                .expect("matrix should be calculated"),
            [0.0, -0.28125, 0.78125, 0.8888889, 0.0, 0.0, 0.0, 0.0, 1.0,]
        );
        assert_eq!(
            coordinate_transformation_matrix(&root, &output, Rotation::ROTATE180)
                .expect("matrix should be calculated"),
            [-0.28125, 0.0, 0.78125, 0.0, -0.8888889, 0.8888889, 0.0, 0.0, 1.0,]
        );
        assert_eq!(
            coordinate_transformation_matrix(&root, &output, Rotation::ROTATE270)
                .expect("matrix should be calculated"),
            [0.0, 0.28125, 0.5, -0.8888889, 0.0, 0.8888889, 0.0, 0.0, 1.0,]
        );
    }

    #[test]
    fn maps_public_output_rotations_to_randr_basic_rotations() {
        assert_eq!(
            output_rotation_to_randr(OutputRotation::Normal),
            Rotation::ROTATE0
        );
        assert_eq!(
            output_rotation_to_randr(OutputRotation::Left),
            Rotation::ROTATE90
        );
        assert_eq!(
            output_rotation_to_randr(OutputRotation::Right),
            Rotation::ROTATE270
        );
        assert_eq!(
            output_rotation_to_randr(OutputRotation::Inverted),
            Rotation::ROTATE180
        );
    }

    #[test]
    fn selected_output_rotation_preserves_reflection_bits() {
        let current = Rotation::ROTATE90 | Rotation::REFLECT_X | Rotation::REFLECT_Y;

        assert_eq!(
            selected_output_rotation(current, Some(OutputRotation::Right)),
            Rotation::ROTATE270 | Rotation::REFLECT_X | Rotation::REFLECT_Y
        );
        assert_eq!(selected_output_rotation(current, None), current);
    }

    #[test]
    fn rejects_coordinate_transformation_matrix_for_invalid_root() {
        let output = Rect::new(0, 0, 1920, 1080);

        assert!(coordinate_transformation_matrix(
            &Rect::new(0, 0, 0, 1080),
            &output,
            Rotation::ROTATE0
        )
        .is_err());
        assert!(coordinate_transformation_matrix(
            &Rect::new(0, 0, 1920, 0),
            &output,
            Rotation::ROTATE0
        )
        .is_err());
    }

    #[test]
    fn selects_only_enabled_touch_devices() {
        assert!(touch_device(&test_xi_device(
            true,
            vec![DeviceClassData::Touch(DeviceClassDataTouch {
                mode: TouchMode::DIRECT,
                num_touches: 10,
            })],
        )));
        assert!(!touch_device(&test_xi_device(
            false,
            vec![DeviceClassData::Touch(DeviceClassDataTouch {
                mode: TouchMode::DIRECT,
                num_touches: 10,
            })],
        )));
        assert!(!touch_device(&test_xi_device(
            true,
            vec![DeviceClassData::Key(DeviceClassDataKey {
                keys: Vec::new()
            })],
        )));
    }

    struct TestMode {
        id: u32,
        width: u16,
        height: u16,
        dot_clock: u32,
        htotal: u16,
        vtotal: u16,
        mode_flags: ModeFlag,
        name_len: u16,
    }

    fn test_mode(spec: TestMode) -> ModeInfo {
        ModeInfo {
            id: spec.id,
            width: spec.width,
            height: spec.height,
            dot_clock: spec.dot_clock,
            hsync_start: 0,
            hsync_end: 0,
            htotal: spec.htotal,
            hskew: 0,
            vsync_start: 0,
            vsync_end: 0,
            vtotal: spec.vtotal,
            name_len: spec.name_len,
            mode_flags: spec.mode_flags,
        }
    }

    fn test_xi_device(enabled: bool, classes: Vec<DeviceClassData>) -> XIDeviceInfo {
        XIDeviceInfo {
            deviceid: DeviceId::from(1_u16),
            type_: DeviceType::SLAVE_POINTER,
            attachment: DeviceId::from(0_u16),
            enabled,
            name: b"test device".to_vec(),
            classes: classes
                .into_iter()
                .map(|data| DeviceClass {
                    len: 2,
                    sourceid: DeviceId::from(1_u16),
                    data,
                })
                .collect(),
        }
    }
}
