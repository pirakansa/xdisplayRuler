use std::io::{self, Write};

use crate::{
    BackendError, ConfiguredBackend, DisplayMonitor, OutputMode, OutputModeSelection,
    WindowGeometryChange, WindowId,
};

const HELP: &str = "\
xdisplay-ruler

Usage:
  xdisplay-ruler [snapshot] [--backend x11]
  xdisplay-ruler watch [--backend x11] [--iterations N]
  xdisplay-ruler modes --output NAME [--backend x11]
  xdisplay-ruler mode --output NAME --width N --height N [--rate HZ] [--backend x11]
  xdisplay-ruler place --window ID --output NAME --fullscreen [--backend x11]
  xdisplay-ruler configure --window ID [--x N] [--y N] [--width N] [--height N] [--backend x11]
  xdisplay-ruler raise --window ID [--backend x11]
  xdisplay-ruler lower --window ID [--backend x11]
  xdisplay-ruler --help
  xdisplay-ruler --version

Commands:
  snapshot  Print one display-state snapshot. This is the default command.
  watch     Keep refreshing and printing display-state snapshots.
  modes     List available modes for an output.
  mode      Change an output mode.
  place     Place a window on an output.
  configure Move or resize a window.
  raise     Raise a window above its siblings.
  lower     Lower a window below its siblings.

Options:
  --backend NAME      Backend to use. Supported: x11.
  --iterations N      Stop watch after N snapshots for tests and diagnostics.
  --output NAME       X11 RandR output name, for example HDMI-2.
  --rate HZ           Refresh rate for mode, for example 60 or 59.94.
  --fullscreen        Resize and move the window to fill the output.
  --window ID         X11 window ID as hex, for example 0x800003.
  --x N               Window X position for configure.
  --y N               Window Y position for configure.
  --width N           Window width for configure.
  --height N          Window height for configure.
";

#[derive(Debug, Eq, PartialEq)]
pub enum CliExit {
    Success,
    UsageError,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Snapshot,
    Watch,
    Modes,
    Mode,
    Place,
    Configure,
    Raise,
    Lower,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CliOptions {
    command: Command,
    backend_name: String,
    iterations: Option<usize>,
    output_name: Option<String>,
    mode_width: Option<u16>,
    mode_height: Option<u16>,
    mode_refresh_millihertz: Option<u32>,
    fullscreen: bool,
    window_id: Option<WindowId>,
    geometry_change: WindowGeometryChange,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: Command::Snapshot,
            backend_name: "x11".to_string(),
            iterations: None,
            output_name: None,
            mode_width: None,
            mode_height: None,
            mode_refresh_millihertz: None,
            fullscreen: false,
            window_id: None,
            geometry_change: WindowGeometryChange::default(),
        }
    }
}

pub fn run<I, S>(
    arguments: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<CliExit>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments = arguments
        .into_iter()
        .map(|argument| argument.as_ref().to_string())
        .collect::<Vec<_>>();

    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        write!(stdout, "{HELP}")?;
        return Ok(CliExit::Success);
    }

    if arguments
        .iter()
        .any(|argument| argument == "--version" || argument == "-V")
    {
        writeln!(stdout, "{}", env!("CARGO_PKG_VERSION"))?;
        return Ok(CliExit::Success);
    }

    let options = match parse_options(&arguments) {
        Ok(options) => options,
        Err(message) => {
            writeln!(stderr, "{message}")?;
            writeln!(stderr, "try --help")?;
            return Ok(CliExit::UsageError);
        }
    };

    match options.command {
        Command::Snapshot => {
            handle_command_result(run_snapshot(&options.backend_name, stdout), stderr)
        }
        Command::Watch => handle_command_result(run_watch(options, stdout), stderr),
        Command::Modes => handle_command_result(run_modes_command(options, stdout), stderr),
        Command::Mode => handle_command_result(run_mode_command(options), stderr),
        Command::Place => handle_command_result(run_place_command(options), stderr),
        Command::Configure => handle_command_result(run_configure_command(options), stderr),
        Command::Raise => {
            handle_command_result(run_stack_command(options, StackCommand::Raise), stderr)
        }
        Command::Lower => {
            handle_command_result(run_stack_command(options, StackCommand::Lower), stderr)
        }
    }
}

fn parse_options(arguments: &[String]) -> Result<CliOptions, String> {
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
            "--fullscreen" => {
                options.fullscreen = true;
            }
            "--window" => {
                let value = next_value(arguments, &mut index, "--window")?;
                options.window_id = Some(parse_window_id(value)?);
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
            argument => return Err(format!("unknown argument: {argument}")),
        }

        index += 1;
    }

    Ok(options)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StackCommand {
    Raise,
    Lower,
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

fn parse_refresh_millihertz(value: &str) -> Result<u32, String> {
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

fn parse_window_id(value: &str) -> Result<WindowId, String> {
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

fn run_snapshot(backend_name: &str, stdout: &mut impl Write) -> Result<(), String> {
    let mut monitor = DisplayMonitor::new(build_backend(backend_name)?);
    monitor.refresh_once().map_err(|error| error.to_string())?;
    write!(stdout, "{}", monitor.status_report()).map_err(|error| error.to_string())
}

fn run_watch(options: CliOptions, stdout: &mut impl Write) -> Result<(), String> {
    let mut monitor = DisplayMonitor::new(build_backend(&options.backend_name)?);
    let mut iteration = 0;

    loop {
        iteration += 1;
        monitor.refresh_once().map_err(|error| error.to_string())?;

        if iteration > 1 {
            writeln!(stdout).map_err(|error| error.to_string())?;
        }

        write!(stdout, "{}", monitor.status_report()).map_err(|error| error.to_string())?;

        if options.iterations == Some(iteration) {
            break;
        }
    }

    Ok(())
}

fn run_stack_command(options: CliOptions, command: StackCommand) -> Result<(), String> {
    let window_id = options
        .window_id
        .ok_or_else(|| "--window is required".to_string())?;
    let backend = build_backend(&options.backend_name)?;

    match command {
        StackCommand::Raise => backend.raise_window(window_id),
        StackCommand::Lower => backend.lower_window(window_id),
    }
    .map_err(|error| error.to_string())
}

fn run_modes_command(options: CliOptions, stdout: &mut impl Write) -> Result<(), String> {
    let output_name = options
        .output_name
        .ok_or_else(|| "--output is required".to_string())?;
    let backend = build_backend(&options.backend_name)?;
    let modes = backend
        .output_modes(&output_name)
        .map_err(|error| error.to_string())?;

    write!(stdout, "{}", modes_report(&output_name, &modes)).map_err(|error| error.to_string())
}

fn run_mode_command(options: CliOptions) -> Result<(), String> {
    let output_name = options
        .output_name
        .ok_or_else(|| "--output is required".to_string())?;
    let selection = OutputModeSelection {
        width: options
            .mode_width
            .ok_or_else(|| "--width is required".to_string())?,
        height: options
            .mode_height
            .ok_or_else(|| "--height is required".to_string())?,
        refresh_millihertz: options.mode_refresh_millihertz,
    };
    let backend = build_backend(&options.backend_name)?;

    backend
        .set_output_mode(&output_name, &selection)
        .map_err(|error| error.to_string())
}

fn run_place_command(options: CliOptions) -> Result<(), String> {
    let window_id = options
        .window_id
        .ok_or_else(|| "--window is required".to_string())?;
    let output_name = options
        .output_name
        .ok_or_else(|| "--output is required".to_string())?;

    if !options.fullscreen {
        return Err("--fullscreen is required for place".to_string());
    }

    let backend = build_backend(&options.backend_name)?;
    backend
        .place_window_fullscreen(window_id, &output_name)
        .map_err(|error| error.to_string())
}

fn run_configure_command(options: CliOptions) -> Result<(), String> {
    let window_id = options
        .window_id
        .ok_or_else(|| "--window is required".to_string())?;

    if options.geometry_change.is_empty() {
        return Err("at least one of --x, --y, --width, or --height is required".to_string());
    }

    let backend = build_backend(&options.backend_name)?;
    backend
        .configure_window(window_id, &options.geometry_change)
        .map_err(|error| error.to_string())
}

fn modes_report(output_name: &str, modes: &[OutputMode]) -> String {
    let mut report = format!(
        "xdisplay-ruler\noutput: {output_name}\nmodes: {}\n",
        modes.len()
    );

    for mode in modes {
        let refresh = mode
            .refresh_millihertz
            .map(format_refresh_millihertz)
            .unwrap_or_else(|| "unknown-rate".to_string());
        let current = if mode.current { " current" } else { "" };
        let preferred = if mode.preferred { " preferred" } else { "" };

        report.push_str(&format!(
            "- {}x{} {} name=\"{}\"{}{}\n",
            mode.width,
            mode.height,
            refresh,
            escape_report_value(&mode.name),
            current,
            preferred
        ));
    }

    report
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

fn escape_report_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn build_backend(name: &str) -> Result<ConfiguredBackend, String> {
    ConfiguredBackend::from_name(name).map_err(|error| match error {
        BackendError::Io(error) => error.to_string(),
        BackendError::UnsupportedName(name) => format!("unsupported backend: {name}"),
    })
}

fn handle_command_result(
    result: Result<(), String>,
    stderr: &mut impl Write,
) -> io::Result<CliExit> {
    match result {
        Ok(()) => Ok(CliExit::Success),
        Err(message) => {
            writeln!(stderr, "{message}")?;
            writeln!(stderr, "try --help")?;
            Ok(CliExit::UsageError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{run, CliExit};
    use crate::OutputMode;

    #[test]
    fn reports_usage_errors_for_unknown_arguments() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["--bad-option"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "unknown argument: --bad-option\ntry --help\n"
        );
    }

    #[test]
    fn accepts_snapshot_command_and_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["snapshot", "--backend", "in-memory"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::Success);
        assert!(stderr.is_empty());
        assert!(String::from_utf8_lossy(&stdout).contains("backend: in-memory\n"));
    }

    #[test]
    fn reports_unsupported_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit =
            run(["--backend", "unsupported"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "unsupported backend: unsupported\ntry --help\n"
        );
    }

    #[test]
    fn limits_watch_iterations() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["watch", "--backend", "in-memory", "--iterations", "2"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::Success);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stdout)
                .matches("xdisplay-ruler")
                .count(),
            2
        );
    }

    #[test]
    fn requires_output_for_modes_command() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["modes"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--output is required\ntry --help\n"
        );
    }

    #[test]
    fn requires_output_width_and_height_for_mode_command() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["mode"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--output is required\ntry --help\n"
        );

        stderr.clear();
        let exit =
            run(["mode", "--output", "HDMI-2"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--width is required\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            ["mode", "--output", "HDMI-2", "--width", "1920"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--height is required\ntry --help\n"
        );
    }

    #[test]
    fn validates_mode_values() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["mode", "--output", "HDMI-2", "--width", "70000"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--width must be at most 65535\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            [
                "mode", "--output", "HDMI-2", "--width", "1920", "--height", "1080", "--rate",
                "59.9400",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--rate must be a positive refresh rate in Hz\ntry --help\n"
        );
    }

    #[test]
    fn rejects_mode_commands_for_in_memory_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["modes", "--backend", "in-memory", "--output", "HDMI-2"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "in-memory backend cannot list X11 output modes\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            [
                "mode",
                "--backend",
                "in-memory",
                "--output",
                "HDMI-2",
                "--width",
                "1920",
                "--height",
                "1080",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "in-memory backend cannot change X11 output modes\ntry --help\n"
        );
    }

    #[test]
    fn parses_refresh_rates_as_millihertz() {
        assert_eq!(super::parse_refresh_millihertz("60"), Ok(60_000));
        assert_eq!(super::parse_refresh_millihertz("59.94"), Ok(59_940));
        assert_eq!(super::parse_refresh_millihertz("59.940"), Ok(59_940));
        assert!(super::parse_refresh_millihertz("0").is_err());
        assert!(super::parse_refresh_millihertz("59.9400").is_err());
        assert!(super::parse_refresh_millihertz("fast").is_err());
    }

    #[test]
    fn renders_modes_report() {
        let report = super::modes_report(
            "HDMI-2",
            &[
                OutputMode {
                    name: "1920x1080".to_string(),
                    width: 1920,
                    height: 1080,
                    refresh_millihertz: Some(60_000),
                    preferred: true,
                    current: true,
                },
                OutputMode {
                    name: "1280\"x720".to_string(),
                    width: 1280,
                    height: 720,
                    refresh_millihertz: Some(59_940),
                    preferred: false,
                    current: false,
                },
            ],
        );

        assert_eq!(
            report,
            concat!(
                "xdisplay-ruler\n",
                "output: HDMI-2\n",
                "modes: 2\n",
                "- 1920x1080 60Hz name=\"1920x1080\" current preferred\n",
                "- 1280x720 59.94Hz name=\"1280\\\"x720\"\n",
            )
        );
    }

    #[test]
    fn requires_window_for_stack_commands() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["raise"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--window is required\ntry --help\n"
        );
    }

    #[test]
    fn rejects_stack_commands_for_in_memory_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["lower", "--backend", "in-memory", "--window", "0x800003"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "in-memory backend cannot change X11 window stacking\ntry --help\n"
        );
    }

    #[test]
    fn requires_output_and_fullscreen_for_place() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["place", "--window", "0x800003"], &mut stdout, &mut stderr)
            .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--output is required\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            ["place", "--window", "0x800003", "--output", "HDMI-2"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--fullscreen is required for place\ntry --help\n"
        );
    }

    #[test]
    fn rejects_place_for_in_memory_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            [
                "place",
                "--backend",
                "in-memory",
                "--window",
                "0x800003",
                "--output",
                "HDMI-2",
                "--fullscreen",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "in-memory backend cannot place X11 windows\ntry --help\n"
        );
    }

    #[test]
    fn requires_window_and_geometry_for_configure() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit =
            run(["configure", "--x", "10"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--window is required\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            ["configure", "--window", "0x800003"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "at least one of --x, --y, --width, or --height is required\ntry --help\n"
        );
    }

    #[test]
    fn validates_configure_geometry_values() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["configure", "--window", "0x800003", "--width", "0"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--width must be a positive integer\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            ["configure", "--window", "0x800003", "--x", "left"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--x must be an integer\ntry --help\n"
        );
    }

    #[test]
    fn rejects_configure_for_in_memory_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            [
                "configure",
                "--backend",
                "in-memory",
                "--window",
                "0x800003",
                "--x",
                "-20",
                "--y",
                "10",
                "--width",
                "480",
                "--height",
                "260",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "in-memory backend cannot configure X11 windows\ntry --help\n"
        );
    }

    #[test]
    fn parses_window_ids_as_hex_or_decimal() {
        assert_eq!(
            super::parse_window_id("0x800003"),
            Ok(crate::WindowId(0x800003))
        );
        assert_eq!(
            super::parse_window_id("8388611"),
            Ok(crate::WindowId(0x800003))
        );
        assert!(super::parse_window_id("not-a-window").is_err());
    }
}
