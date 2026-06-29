use std::io;

use x11rb::protocol::randr;

use crate::{OutputModeSelection, OutputRotation};

use super::types::{ScreenBounds, ScreenSize, X11ModeInfo};

pub(super) fn ensure_connected_output(
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

pub(super) fn mode_infos(resources: &randr::GetScreenResourcesCurrentReply) -> Vec<X11ModeInfo> {
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

pub(super) fn refresh_millihertz(mode: &randr::ModeInfo) -> Option<u32> {
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

pub(super) fn refresh_matches(actual: Option<u32>, expected: Option<u32>) -> bool {
    match (actual, expected) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual.abs_diff(expected) <= 500,
        (None, Some(_)) => false,
    }
}

pub(super) fn mode_not_found_message(selection: &OutputModeSelection) -> String {
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

pub(super) fn set_config_status_label(status: randr::SetConfig) -> &'static str {
    match status {
        randr::SetConfig::SUCCESS => "success",
        randr::SetConfig::INVALID_CONFIG_TIME => "invalid config time",
        randr::SetConfig::INVALID_TIME => "invalid time",
        randr::SetConfig::FAILED => "failed",
        _ => "unknown",
    }
}

pub(super) fn selected_output_rotation(
    current: randr::Rotation,
    selected: Option<OutputRotation>,
) -> randr::Rotation {
    let basic = selected.map_or_else(|| basic_rotation(current), output_rotation_to_randr);
    let reflection = u16::from(current)
        & (u16::from(randr::Rotation::REFLECT_X) | u16::from(randr::Rotation::REFLECT_Y));

    randr::Rotation::from(u16::from(basic) | reflection)
}

pub(super) fn output_rotation_to_randr(rotation: OutputRotation) -> randr::Rotation {
    match rotation {
        OutputRotation::Normal => randr::Rotation::ROTATE0,
        OutputRotation::Left => randr::Rotation::ROTATE90,
        OutputRotation::Right => randr::Rotation::ROTATE270,
        OutputRotation::Inverted => randr::Rotation::ROTATE180,
    }
}

pub(super) fn basic_rotation(rotation: randr::Rotation) -> randr::Rotation {
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

pub(super) fn transformed_mode_size(mode: &X11ModeInfo, rotation: randr::Rotation) -> (u16, u16) {
    match basic_rotation(rotation) {
        randr::Rotation::ROTATE90 | randr::Rotation::ROTATE270 => (mode.height, mode.width),
        _ => (mode.width, mode.height),
    }
}

pub(super) fn requested_mode_size(
    width: u16,
    height: u16,
    rotation: randr::Rotation,
) -> (u16, u16) {
    match basic_rotation(rotation) {
        randr::Rotation::ROTATE90 | randr::Rotation::ROTATE270 => (height, width),
        _ => (width, height),
    }
}

pub(super) fn screen_size_for_bounds(bounds: &[ScreenBounds]) -> io::Result<ScreenSize> {
    let mut width = 1_i64;
    let mut height = 1_i64;

    for bound in bounds {
        width = width.max(i64::from(bound.x) + i64::from(bound.width));
        height = height.max(i64::from(bound.y) + i64::from(bound.height));
    }

    if width <= 0 || height <= 0 || width > i64::from(u16::MAX) || height > i64::from(u16::MAX) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("required RandR screen size is out of range: {width}x{height}"),
        ));
    }

    Ok(ScreenSize {
        width: width as u16,
        height: height as u16,
    })
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

#[cfg(test)]
mod tests {
    use super::{
        mode_infos, output_rotation_to_randr, refresh_millihertz, requested_mode_size,
        screen_size_for_bounds, selected_output_rotation, transformed_mode_size, ScreenBounds,
        ScreenSize, X11ModeInfo,
    };
    use crate::OutputRotation;
    use x11rb::protocol::randr::{GetScreenResourcesCurrentReply, ModeFlag, ModeInfo, Rotation};

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
    fn rotated_modes_swap_screen_size_axes() {
        let mode = X11ModeInfo {
            id: 1,
            name: "1920x1080".to_string(),
            width: 1920,
            height: 1080,
            refresh_millihertz: Some(60_000),
        };

        assert_eq!(
            transformed_mode_size(&mode, Rotation::ROTATE0),
            (1920, 1080)
        );
        assert_eq!(
            transformed_mode_size(&mode, Rotation::ROTATE90),
            (1080, 1920)
        );
        assert_eq!(
            transformed_mode_size(&mode, Rotation::ROTATE180),
            (1920, 1080)
        );
        assert_eq!(
            transformed_mode_size(&mode, Rotation::ROTATE270),
            (1080, 1920)
        );
    }

    #[test]
    fn rotated_requested_size_maps_to_unrotated_randr_mode_size() {
        assert_eq!(
            requested_mode_size(1080, 1920, Rotation::ROTATE90),
            (1920, 1080)
        );
        assert_eq!(
            requested_mode_size(1080, 1920, Rotation::ROTATE270),
            (1920, 1080)
        );
        assert_eq!(
            requested_mode_size(1920, 1080, Rotation::ROTATE0),
            (1920, 1080)
        );
        assert_eq!(
            requested_mode_size(1920, 1080, Rotation::ROTATE180),
            (1920, 1080)
        );
    }

    #[test]
    fn screen_size_covers_all_crtc_bounds() {
        let size = screen_size_for_bounds(&[
            ScreenBounds {
                x: 0,
                y: 0,
                width: 1080,
                height: 1920,
            },
            ScreenBounds {
                x: 1080,
                y: 0,
                width: 1280,
                height: 720,
            },
        ])
        .expect("screen size should be valid");

        assert_eq!(
            size,
            ScreenSize {
                width: 2360,
                height: 1920,
            }
        );
    }

    #[test]
    fn pre_rotation_screen_size_expands_without_shrinking_either_axis() {
        let current = ScreenSize {
            width: 1920,
            height: 1080,
        };
        let rotated = ScreenSize {
            width: 1080,
            height: 1920,
        };

        assert_eq!(
            current.expanded_to(rotated),
            ScreenSize {
                width: 1920,
                height: 1920,
            }
        );
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
}
