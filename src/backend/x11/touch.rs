use std::io;

use x11rb::protocol::{
    randr,
    xinput::{DeviceClassData, XIDeviceInfo},
};

use crate::Rect;

use super::mode::basic_rotation;

pub(super) fn coordinate_transformation_matrix(
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

pub(super) fn touch_device(device: &XIDeviceInfo) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{coordinate_transformation_matrix, touch_device};
    use crate::Rect;
    use x11rb::protocol::{
        randr::Rotation,
        xinput::{
            DeviceClass, DeviceClassData, DeviceClassDataKey, DeviceClassDataTouch, DeviceId,
            DeviceType, TouchMode, XIDeviceInfo,
        },
    };

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
