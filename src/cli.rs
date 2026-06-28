use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use crate::{BackendError, ConfiguredBackend, DisplayMonitor};

const HELP: &str = "\
display-ruler

Usage:
  display-ruler [snapshot] [--backend in-memory]
  display-ruler watch [--backend in-memory] [--interval-ms MS] [--iterations N]
  display-ruler --help
  display-ruler --version

Commands:
  snapshot  Print one display-state snapshot. This is the default command.
  watch     Keep refreshing and printing display-state snapshots.

Options:
  --backend NAME      Backend to use. Supported: in-memory.
  --interval-ms MS    Delay between watch refreshes. Default: 1000.
  --iterations N      Stop watch mode after N refreshes.
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CliOptions {
    command: Command,
    backend_name: String,
    interval: Duration,
    iterations: Option<usize>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: Command::Snapshot,
            backend_name: "in-memory".to_string(),
            interval: Duration::from_millis(1_000),
            iterations: None,
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
            "--interval-ms" => {
                let value = next_value(arguments, &mut index, "--interval-ms")?;
                options.interval =
                    Duration::from_millis(parse_non_zero_u64(value, "--interval-ms")?);
            }
            "--iterations" => {
                let value = next_value(arguments, &mut index, "--iterations")?;
                options.iterations = Some(parse_non_zero_usize(value, "--iterations")?);
            }
            argument => return Err(format!("unknown argument: {argument}")),
        }

        index += 1;
    }

    Ok(options)
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

fn parse_non_zero_u64(value: &str, option_name: &str) -> Result<u64, String> {
    let value = value
        .parse::<u64>()
        .map_err(|_| format!("{option_name} must be a positive integer"))?;

    if value == 0 {
        return Err(format!("{option_name} must be a positive integer"));
    }

    Ok(value)
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

        thread::sleep(options.interval);
    }

    Ok(())
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
    fn reports_unavailable_x11_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["--backend", "x11"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            concat!(
                "x11 backend requires an X11 client implementation that is not available in this build\n",
                "try --help\n"
            )
        );
    }

    #[test]
    fn limits_watch_iterations() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["watch", "--iterations", "2", "--interval-ms", "1"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::Success);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stdout)
                .matches("display-ruler")
                .count(),
            2
        );
    }
}
