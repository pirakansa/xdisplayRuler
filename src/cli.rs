use std::io::{self, Write};

use crate::{BackendError, ConfiguredBackend, DisplayMonitor, WindowId};

const HELP: &str = "\
xdisplay-ruler

Usage:
  xdisplay-ruler [snapshot] [--backend x11]
  xdisplay-ruler watch [--backend x11] [--iterations N]
  xdisplay-ruler place --window ID --output NAME --fullscreen [--backend x11]
  xdisplay-ruler raise --window ID [--backend x11]
  xdisplay-ruler lower --window ID [--backend x11]
  xdisplay-ruler --help
  xdisplay-ruler --version

Commands:
  snapshot  Print one display-state snapshot. This is the default command.
  watch     Keep refreshing and printing display-state snapshots.
  place     Place a window on an output.
  raise     Raise a window above its siblings.
  lower     Lower a window below its siblings.

Options:
  --backend NAME      Backend to use. Supported: x11.
  --iterations N      Stop watch after N snapshots for tests and diagnostics.
  --output NAME       X11 RandR output name, for example HDMI-2.
  --fullscreen        Resize and move the window to fill the output.
  --window ID         X11 window ID as hex, for example 0x800003.
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
    Place,
    Raise,
    Lower,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CliOptions {
    command: Command,
    backend_name: String,
    iterations: Option<usize>,
    output_name: Option<String>,
    fullscreen: bool,
    window_id: Option<WindowId>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: Command::Snapshot,
            backend_name: "x11".to_string(),
            iterations: None,
            output_name: None,
            fullscreen: false,
            window_id: None,
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
        Command::Place => handle_command_result(run_place_command(options), stderr),
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
            "place" => {
                options.command = Command::Place;
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
