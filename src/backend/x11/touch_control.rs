use std::io;

use x11rb::protocol::{
    randr,
    xinput::{ConnectionExt as XinputConnectionExt, Device, XIChangePropertyAux},
    xproto::{ConnectionExt as XprotoConnection, PropMode},
};

use crate::Rect;

use super::{
    touch::{coordinate_transformation_matrix, touch_device},
    X11Backend, COORDINATE_TRANSFORMATION_MATRIX, FLOAT_ATOM,
};

impl X11Backend {
    pub(super) fn remap_touch_devices_to_output(
        &self,
        output_name: &str,
        rotation: randr::Rotation,
    ) -> io::Result<()> {
        let root_geometry = self
            .connection
            .get_geometry(self.root_window())
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;
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
            .map_err(super::to_io_error)?
            .reply()
            .map_err(super::to_io_error)?;

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
                .map_err(super::to_io_error)?
                .check()
                .map_err(super::to_io_error)?;
        }

        Ok(())
    }
}
