use crate::{OutputRotation, WindowGeometryChange, WindowId};

#[derive(Debug, Eq, PartialEq)]
pub enum CliExit {
    Success,
    UsageError,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Command {
    Snapshot,
    Watch,
    Modes,
    Mode,
    Enforce,
    Place,
    Configure,
    Raise,
    Lower,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CliOptions {
    pub(super) command: Command,
    pub(super) backend_name: String,
    pub(super) iterations: Option<usize>,
    pub(super) output_name: Option<String>,
    pub(super) layout_path: Option<String>,
    pub(super) once: bool,
    pub(super) dry_run: bool,
    pub(super) interval_millis: usize,
    pub(super) mode_width: Option<u16>,
    pub(super) mode_height: Option<u16>,
    pub(super) mode_refresh_millihertz: Option<u32>,
    pub(super) mode_rotation: Option<OutputRotation>,
    pub(super) fullscreen: bool,
    pub(super) window_selector: Option<WindowSelector>,
    pub(super) geometry_change: WindowGeometryChange,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: Command::Snapshot,
            backend_name: "x11".to_string(),
            iterations: None,
            output_name: None,
            layout_path: None,
            once: false,
            dry_run: false,
            interval_millis: 1000,
            mode_width: None,
            mode_height: None,
            mode_refresh_millihertz: None,
            mode_rotation: None,
            fullscreen: false,
            window_selector: None,
            geometry_change: WindowGeometryChange::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum WindowSelector {
    Id(WindowId),
    Title(String),
    Class(String),
    Instance(String),
}

pub(super) fn parse_options(arguments: &[String]) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut index = 0;

    if let Some(command) = arguments.first() {
        match command.as_str() {
            "snapshot" => {
                options.command = Command::Snapshot;
                index = 1;
            }
            "watch" => {
                options.command = Command::Watch;
                index = 1;
            }
            "modes" => {
                options.command = Command::Modes;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            "mode" => {
                options.command = Command::Mode;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            "enforce" => {
                options.command = Command::Enforce;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            "place" => {
                options.command = Command::Place;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            "configure" => {
                options.command = Command::Configure;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            "raise" => {
                options.command = Command::Raise;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            "lower" => {
                options.command = Command::Lower;
                options.backend_name = "x11".to_string();
                index = 1;
            }
            _ if command.starts_with('-') => {}
            _ => return Err(format!("unknown command: {command}")),
        }
    }

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--backend" => {
                let value = next_value(arguments, &mut index, "--backend")?;
                validate_backend_name(value)?;
                options.backend_name = value.to_string();
            }
            "--iterations" => {
                let value = next_value(arguments, &mut index, "--iterations")?;
                options.iterations = Some(parse_non_zero_usize(value, "--iterations")?);
            }
            "--output" => {
                let value = next_value(arguments, &mut index, "--output")?;
                options.output_name = Some(value.to_string());
            }
            "--layout" => {
                let value = next_value(arguments, &mut index, "--layout")?;
                options.layout_path = Some(value.to_string());
            }
            "--once" => {
                options.once = true;
            }
            "--dry-run" => {
                options.dry_run = true;
            }
            "--interval" => {
                let value = next_value(arguments, &mut index, "--interval")?;
                options.interval_millis = parse_non_zero_usize(value, "--interval")?;
            }
            "--fullscreen" => {
                options.fullscreen = true;
            }
            "--window" => {
                let value = next_value(arguments, &mut index, "--window")?;
                options.set_window_selector(WindowSelector::Id(parse_window_id(value)?))?;
            }
            "--window-title" => {
                let value = next_value(arguments, &mut index, "--window-title")?;
                options.set_window_selector(WindowSelector::Title(value.to_string()))?;
            }
            "--window-class" => {
                let value = next_value(arguments, &mut index, "--window-class")?;
                options.set_window_selector(WindowSelector::Class(value.to_string()))?;
            }
            "--window-instance" => {
                let value = next_value(arguments, &mut index, "--window-instance")?;
                options.set_window_selector(WindowSelector::Instance(value.to_string()))?;
            }
            "--x" => {
                let value = next_value(arguments, &mut index, "--x")?;
                options.geometry_change.x = Some(parse_i32(value, "--x")?);
            }
            "--y" => {
                let value = next_value(arguments, &mut index, "--y")?;
                options.geometry_change.y = Some(parse_i32(value, "--y")?);
            }
            "--width" => {
                let value = next_value(arguments, &mut index, "--width")?;
                let width = parse_positive_u32(value, "--width")?;
                options.geometry_change.width = Some(width);
                options.mode_width = Some(parse_positive_u16(value, "--width")?);
            }
            "--height" => {
                let value = next_value(arguments, &mut index, "--height")?;
                let height = parse_positive_u32(value, "--height")?;
                options.geometry_change.height = Some(height);
                options.mode_height = Some(parse_positive_u16(value, "--height")?);
            }
            "--rate" => {
                let value = next_value(arguments, &mut index, "--rate")?;
                options.mode_refresh_millihertz = Some(parse_refresh_millihertz(value)?);
            }
            "--rotate" => {
                let value = next_value(arguments, &mut index, "--rotate")?;
                options.mode_rotation = Some(parse_output_rotation(value)?);
            }
            argument => return Err(format!("unknown argument: {argument}")),
        }

        index += 1;
    }

    Ok(options)
}

impl CliOptions {
    fn set_window_selector(&mut self, selector: WindowSelector) -> Result<(), String> {
        if self.window_selector.is_some() {
            return Err(
                "--window, --window-title, --window-class, and --window-instance are mutually exclusive"
                    .to_string(),
            );
        }

        self.window_selector = Some(selector);
        Ok(())
    }
}

fn next_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option_name: &str,
) -> Result<&'a str, String> {
    *index += 1;
    arguments
        .get(*index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option_name} requires a value"))
}

fn validate_backend_name(value: &str) -> Result<(), String> {
    match value {
        "in-memory" | "x11" | "xorg" => Ok(()),
        _ => Err(format!("unsupported backend: {value}")),
    }
}

fn parse_non_zero_usize(value: &str, option_name: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|_| format!("{option_name} must be a positive integer"))?;

    if value == 0 {
        return Err(format!("{option_name} must be a positive integer"));
    }

    Ok(value)
}

fn parse_i32(value: &str, option_name: &str) -> Result<i32, String> {
    value
        .parse::<i32>()
        .map_err(|_| format!("{option_name} must be an integer"))
}

fn parse_positive_u32(value: &str, option_name: &str) -> Result<u32, String> {
    let value = value
        .parse::<u32>()
        .map_err(|_| format!("{option_name} must be a positive integer"))?;

    if value == 0 {
        return Err(format!("{option_name} must be a positive integer"));
    }

    Ok(value)
}

fn parse_positive_u16(value: &str, option_name: &str) -> Result<u16, String> {
    let value = parse_positive_u32(value, option_name)?;

    u16::try_from(value).map_err(|_| format!("{option_name} must be at most {}", u16::MAX))
}

pub(super) fn parse_refresh_millihertz(value: &str) -> Result<u32, String> {
    let (whole, fraction) = value
        .split_once('.')
        .map_or((value, ""), |(whole, fraction)| (whole, fraction));

    if whole.is_empty()
        || !whole.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
        || fraction.len() > 3
    {
        return Err("--rate must be a positive refresh rate in Hz".to_string());
    }

    let whole = whole
        .parse::<u32>()
        .map_err(|_| "--rate must be a positive refresh rate in Hz".to_string())?;
    let fraction = format!("{fraction:0<3}")
        .parse::<u32>()
        .map_err(|_| "--rate must be a positive refresh rate in Hz".to_string())?;
    let rate = whole
        .checked_mul(1000)
        .and_then(|whole| whole.checked_add(fraction))
        .ok_or_else(|| "--rate must be a positive refresh rate in Hz".to_string())?;

    if rate == 0 {
        return Err("--rate must be a positive refresh rate in Hz".to_string());
    }

    Ok(rate)
}

pub(super) fn parse_output_rotation(value: &str) -> Result<OutputRotation, String> {
    match value {
        "normal" => Ok(OutputRotation::Normal),
        "left" => Ok(OutputRotation::Left),
        "right" => Ok(OutputRotation::Right),
        "inverted" => Ok(OutputRotation::Inverted),
        _ => Err("--rotate must be one of: normal, left, right, inverted".to_string()),
    }
}

pub(super) fn parse_window_id(value: &str) -> Result<WindowId, String> {
    let normalized = value.trim();
    let parsed = normalized
        .strip_prefix("0x")
        .or_else(|| normalized.strip_prefix("0X"))
        .map_or_else(
            || normalized.parse::<u64>(),
            |hex| u64::from_str_radix(hex, 16),
        )
        .map_err(|_| format!("--window must be an X11 window id, got: {value}"))?;

    Ok(WindowId(parsed))
}
