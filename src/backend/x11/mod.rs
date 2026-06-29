use std::io;

mod event;
mod mode;
mod touch;
mod types;
mod window;

use x11rb::{
    connection::Connection,
    protocol::{
        randr::{self, ConnectionExt as RandrConnectionExt, GetScreenResourcesCurrentReply},
        xinput::{ConnectionExt as XinputConnectionExt, Device, XIChangePropertyAux},
        xproto::{
            ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as XprotoConnection,
            EventMask, MapState, PropMode, StackMode,
        },
    },
    rust_connection::RustConnection,
};

use crate::{
    DisplayBackend, DisplayEvent, OutputMode, OutputModeChange, OutputModeSelection, Rect,
    WindowGeometryChange, WindowId, WindowInfo,
};

use event::is_relevant_event;
use mode::{
    ensure_connected_output, mode_infos, mode_not_found_message, refresh_matches,
    requested_mode_size, screen_size_for_bounds, selected_output_rotation, set_config_status_label,
    transformed_mode_size,
};
use touch::{coordinate_transformation_matrix, touch_device};
use types::{
    ScreenBounds, ScreenSize, SelectedCrtcConfig, X11ModeInfo, X11OutputSnapshot, X11Snapshot,
    X11WindowClass, X11WindowSnapshot,
};
use window::{text_property_value, window_class_value, x11_window_id};

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

    pub fn stack_window_above(&self, id: WindowId, sibling: WindowId) -> io::Result<()> {
        let window = x11_window_id(id)?;
        let sibling = x11_window_id(sibling)?;
        let changes = ConfigureWindowAux::new()
            .sibling(sibling)
            .stack_mode(StackMode::ABOVE);

        self.connection
            .configure_window(window, &changes)
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)?;
        self.connection.flush().map_err(to_io_error)
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
        let mut selected = self.selected_crtc_config(output_name, selection)?;
        let current_screen_size = self.root_screen_size()?;
        let pre_config_screen_size = current_screen_size.expanded_to(selected.screen_size);

        if pre_config_screen_size != current_screen_size {
            self.set_root_screen_size(pre_config_screen_size)?;
            selected = self.selected_crtc_config(output_name, selection)?;
        }

        let reply = self
            .connection
            .randr_set_crtc_config(
                selected.crtc,
                0,
                selected.config_timestamp,
                selected.x,
                selected.y,
                selected.mode,
                selected.rotation,
                &selected.outputs,
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

        if selected.screen_size != pre_config_screen_size {
            self.set_root_screen_size(selected.screen_size)?;
        }

        let mut change = OutputModeChange::default();
        if let Err(error) = self.remap_touch_devices_to_output(output_name, selected.rotation) {
            change.warnings.push(format!(
                "output mode changed, but touch remapping failed: {error}"
            ));
        }

        self.connection.flush().map_err(to_io_error)?;
        Ok(change)
    }

    fn selected_crtc_config(
        &self,
        output_name: &str,
        selection: &OutputModeSelection,
    ) -> io::Result<SelectedCrtcConfig> {
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
        let screen_size = self.selected_screen_size(
            &resources,
            output.crtc,
            crtc.x,
            crtc.y,
            &selected_mode,
            selected_rotation,
        )?;

        Ok(SelectedCrtcConfig {
            crtc: output.crtc,
            config_timestamp: resources.config_timestamp,
            x: crtc.x,
            y: crtc.y,
            mode: selected_mode.id,
            rotation: selected_rotation,
            outputs,
            screen_size,
        })
    }

    fn selected_screen_size(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        selected_crtc: randr::Crtc,
        selected_x: i16,
        selected_y: i16,
        selected_mode: &X11ModeInfo,
        selected_rotation: randr::Rotation,
    ) -> io::Result<ScreenSize> {
        let mut bounds = Vec::new();

        for crtc_id in &resources.crtcs {
            let crtc = self
                .connection
                .randr_get_crtc_info(*crtc_id, resources.config_timestamp)
                .map_err(to_io_error)?
                .reply()
                .map_err(to_io_error)?;

            if *crtc_id == selected_crtc {
                let (width, height) = transformed_mode_size(selected_mode, selected_rotation);
                bounds.push(ScreenBounds {
                    x: i32::from(selected_x),
                    y: i32::from(selected_y),
                    width: u32::from(width),
                    height: u32::from(height),
                });
            } else if crtc.mode != 0 {
                bounds.push(ScreenBounds {
                    x: i32::from(crtc.x),
                    y: i32::from(crtc.y),
                    width: u32::from(crtc.width),
                    height: u32::from(crtc.height),
                });
            }
        }

        screen_size_for_bounds(&bounds)
    }

    fn root_screen_size(&self) -> io::Result<ScreenSize> {
        let root = self
            .connection
            .get_geometry(self.root_window())
            .map_err(to_io_error)?
            .reply()
            .map_err(to_io_error)?;

        Ok(ScreenSize {
            width: root.width,
            height: root.height,
        })
    }

    fn set_root_screen_size(&self, size: ScreenSize) -> io::Result<()> {
        let screen = &self.connection.setup().roots[self.screen_index];

        self.connection
            .randr_set_screen_size(
                self.root_window(),
                size.width,
                size.height,
                u32::from(screen.width_in_millimeters),
                u32::from(screen.height_in_millimeters),
            )
            .map_err(to_io_error)?
            .check()
            .map_err(to_io_error)
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
        let selected_rotation = selected_output_rotation(crtc.rotation, selection.rotation);
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

        let (mode_width, mode_height) = requested_mode_size(width, height, selected_rotation);
        let mode_infos = mode_infos(resources);
        let mut candidates = output
            .modes
            .iter()
            .filter_map(|mode| mode_infos.iter().find(|info| info.id == *mode))
            .filter(|mode| mode.width == mode_width && mode.height == mode_height)
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

fn to_io_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}
