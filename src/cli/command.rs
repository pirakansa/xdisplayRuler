use std::io::{self, Write};

use crate::{
    enforce::{self, EnforceOptions},
    report::escape_value,
    BackendError, ConfiguredBackend, DisplayMonitor, WindowId, WindowInfo,
};

use super::options::{CliExit, CliOptions, WindowSelector};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StackCommand {
    Raise,
    Lower,
}

pub(super) fn run_snapshot(backend_name: &str, stdout: &mut impl Write) -> Result<(), String> {
    let mut monitor = DisplayMonitor::new(build_backend(backend_name)?);
    monitor.refresh_once().map_err(|error| error.to_string())?;
    write!(stdout, "{}", monitor.status_report()).map_err(|error| error.to_string())
}

pub(super) fn run_watch(options: CliOptions, stdout: &mut impl Write) -> Result<(), String> {
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

pub(super) fn run_enforce_command(
    options: CliOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let layout_path = options
        .layout_path
        .ok_or_else(|| "--layout is required".to_string())?;

    enforce::run(
        EnforceOptions {
            backend_name: options.backend_name,
            layout_path,
            once: options.once,
            dry_run: options.dry_run,
            interval_millis: options.interval_millis,
        },
        stdout,
        stderr,
        build_backend,
    )
}

pub(super) fn run_stack_command(options: CliOptions, command: StackCommand) -> Result<(), String> {
    let selector = required_window_selector(&options)?;
    let backend = build_backend(&options.backend_name)?;
    let window_id = resolve_window_selector(&backend, &selector)?;

    match command {
        StackCommand::Raise => backend.raise_window(window_id),
        StackCommand::Lower => backend.lower_window(window_id),
    }
    .map_err(|error| error.to_string())
}

pub(super) fn run_place_command(options: CliOptions) -> Result<(), String> {
    let selector = required_window_selector(&options)?;
    let output_name = options
        .output_name
        .ok_or_else(|| "--output is required".to_string())?;

    if !options.fullscreen {
        return Err("--fullscreen is required for place".to_string());
    }

    let backend = build_backend(&options.backend_name)?;
    let window_id = resolve_window_selector(&backend, &selector)?;
    backend
        .place_window_fullscreen(window_id, &output_name)
        .map_err(|error| error.to_string())
}

pub(super) fn run_configure_command(options: CliOptions) -> Result<(), String> {
    let selector = required_window_selector(&options)?;

    if options.geometry_change.is_empty() {
        return Err("at least one of --x, --y, --width, or --height is required".to_string());
    }

    let backend = build_backend(&options.backend_name)?;
    let window_id = resolve_window_selector(&backend, &selector)?;
    backend
        .configure_window(window_id, &options.geometry_change)
        .map_err(|error| error.to_string())
}

fn required_window_selector(options: &CliOptions) -> Result<WindowSelector, String> {
    options.window_selector.clone().ok_or_else(|| {
        "--window, --window-title, --window-class, or --window-instance is required".to_string()
    })
}

fn resolve_window_selector(
    backend: &ConfiguredBackend,
    selector: &WindowSelector,
) -> Result<WindowId, String> {
    match selector {
        WindowSelector::Id(id) => Ok(*id),
        WindowSelector::Title(title) => resolve_window_from_list(
            &backend.windows().map_err(|error| error.to_string())?,
            selector,
            title,
        ),
        WindowSelector::Class(class_name) => {
            let windows = backend.windows().map_err(|error| error.to_string())?;
            resolve_window_from_list(&windows, selector, class_name)
        }
        WindowSelector::Instance(instance_name) => {
            let windows = backend.windows().map_err(|error| error.to_string())?;
            resolve_window_from_list(&windows, selector, instance_name)
        }
    }
}

pub(super) fn resolve_window_from_list(
    windows: &[WindowInfo],
    selector: &WindowSelector,
    value: &str,
) -> Result<WindowId, String> {
    let matches = windows
        .iter()
        .filter(|window| window.mapped && window_matches_selector(window, selector))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(format!("window not found: {value}")),
        [window] => Ok(window.id),
        _ => Err(ambiguous_window_message(value, &matches)),
    }
}

fn window_matches_selector(window: &WindowInfo, selector: &WindowSelector) -> bool {
    match selector {
        WindowSelector::Id(id) => window.id == *id,
        WindowSelector::Title(title) => window.title.as_deref() == Some(title.as_str()),
        WindowSelector::Class(class_name) => {
            window.class_name.as_deref() == Some(class_name.as_str())
        }
        WindowSelector::Instance(instance_name) => {
            window.instance_name.as_deref() == Some(instance_name.as_str())
        }
    }
}

fn ambiguous_window_message(value: &str, windows: &[&WindowInfo]) -> String {
    let mut message = format!("window selector is ambiguous: {value}");

    for window in windows {
        message.push_str(&format!(
            "\n- {} title=\"{}\" class=\"{}\" instance=\"{}\"",
            window.id,
            escape_value(window.title.as_deref().unwrap_or("")),
            escape_value(window.class_name.as_deref().unwrap_or("")),
            escape_value(window.instance_name.as_deref().unwrap_or(""))
        ));
    }

    message
}

fn build_backend(name: &str) -> Result<ConfiguredBackend, String> {
    ConfiguredBackend::from_name(name).map_err(|error| match error {
        BackendError::Io(error) => error.to_string(),
        BackendError::UnsupportedName(name) => format!("unsupported backend: {name}"),
    })
}

pub(super) fn handle_command_result(
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
