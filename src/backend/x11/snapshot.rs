use std::io;

use x11rb::{
    connection::Connection,
    protocol::{
        randr::{self, ConnectionExt as RandrConnectionExt, GetScreenResourcesCurrentReply},
        xproto::{ConnectionExt as XprotoConnection, MapState},
    },
};

use crate::{DisplayEvent, Rect, WindowId, WindowInfo};

use super::{
    types::{X11OutputSnapshot, X11Snapshot, X11WindowClass, X11WindowSnapshot},
    window::{text_property_value, window_class_value},
    X11Backend,
};

impl X11Backend {
    pub(crate) fn snapshot_events(&self) -> io::Result<Vec<DisplayEvent>> {
        Ok(self.snapshot()?.into_events())
    }

    pub fn windows(&self) -> io::Result<Vec<WindowInfo>> {
        Ok(self
            .window_snapshots()?
            .into_iter()
            .map(X11WindowSnapshot::into_window_info)
            .collect())
    }

    pub(super) fn output_snapshots(&self) -> io::Result<Vec<X11OutputSnapshot>> {
        let root = self.root_window();
        let resources = self
            .connection
            .randr_get_screen_resources_current(root)
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;

        resources
            .outputs
            .iter()
            .map(|output| self.output_snapshot(&resources, *output))
            .collect()
    }

    fn snapshot(&self) -> io::Result<X11Snapshot> {
        Ok(X11Snapshot {
            outputs: self.output_snapshots()?,
            windows: self.window_snapshots()?,
            focused_window: self.focused_window()?,
        })
    }

    fn output_snapshot(
        &self,
        resources: &GetScreenResourcesCurrentReply,
        output: randr::Output,
    ) -> io::Result<X11OutputSnapshot> {
        let info = self
            .connection
            .randr_get_output_info(output, resources.config_timestamp)
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;
        let name = String::from_utf8_lossy(&info.name).into_owned();

        if info.connection != randr::Connection::CONNECTED || info.crtc == 0 {
            return Ok(X11OutputSnapshot::disconnected(name));
        }

        let crtc = self
            .connection
            .randr_get_crtc_info(info.crtc, resources.config_timestamp)
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;
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
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;
        let mut windows = Vec::new();

        for window in tree.children {
            let attributes = match self
                .connection
                .get_window_attributes(window)
                .map_err(super::to_io_error)?
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
                .map_err(super::to_io_error)?
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

    fn focused_window(&self) -> io::Result<Option<WindowId>> {
        if let Ok(focus) = self
            .connection
            .get_input_focus()
            .map_err(super::to_io_error)?
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
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;

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
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;

        if property.value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(property.value))
        }
    }

    pub(super) fn wait_for_relevant_event(&self) -> io::Result<()> {
        loop {
            let event = self
                .connection
                .wait_for_event()
                .map_err(super::to_io_error)?;
            if super::is_relevant_event(&event) {
                return Ok(());
            }
        }
    }
}
