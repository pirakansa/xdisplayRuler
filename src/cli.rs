use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

use crate::{
    build_enforcement_plan, BackendError, ConfiguredBackend, DisplayMonitor, DisplayState,
    EnforcementMode, EnforcementPlan, LayoutOperation, LayoutPolicy, OutputMode, OutputModeChange,
    OutputModeSelection, OutputRotation, WindowGeometryChange, WindowId, WindowInfo,
};

const HELP: &str = "\
xdisplay-ruler

Overview:
  Inspect Xorg display state, list or change RandR output modes, and send
  low-level X11 requests to move, resize, raise, lower, or place windows.

Quick Start:
  xdisplay-ruler
  xdisplay-ruler snapshot --backend x11
  xdisplay-ruler raise --window-class Gnome-terminal

Usage:
  Snapshot:
    xdisplay-ruler [snapshot] [--backend NAME]
    xdisplay-ruler watch [--backend NAME] [--iterations N]

  Output Modes:
    xdisplay-ruler modes --output NAME [--backend x11]
    xdisplay-ruler mode --output NAME [--width N --height N] [--rate HZ] [--rotate DIR] [--backend x11]

  Window Control:
    xdisplay-ruler enforce --layout FILE [--once] [--dry-run] [--interval MS] [--backend x11]
    xdisplay-ruler raise WINDOW_SELECTOR [--backend x11]
    xdisplay-ruler lower WINDOW_SELECTOR [--backend x11]
    xdisplay-ruler configure WINDOW_SELECTOR [--x N] [--y N] [--width N] [--height N] [--backend x11]
    xdisplay-ruler place WINDOW_SELECTOR --output NAME --fullscreen [--backend x11]

  Other:
    xdisplay-ruler --help
    xdisplay-ruler --version

Window Selector:
  Use exactly one selector with raise, lower, configure, or place.
    --window ID             X11 window ID, for example 0x800003.
    --window-title NAME     Exact X11 window title.
    --window-class NAME     Exact WM_CLASS class name.
    --window-instance NAME  Exact WM_CLASS instance name.

Commands:
  snapshot  Print one display-state snapshot. This is the default command.
  watch     Keep refreshing and printing display-state snapshots.
  modes     List available modes for an output.
  mode      Change an output mode.
  enforce   Keep layout-defined windows fitted to their output.
  place     Place a window on an output.
  configure Move or resize a window.
  raise     Raise a window above its siblings.
  lower     Lower a window below its siblings.

Global Options:
  --backend NAME      Backend to use. Supported: x11, xorg, in-memory.
  --iterations N      Stop watch after N snapshots. Must be positive.
  --layout FILE       Layout JSON file for enforce.
  --once              Apply enforce once and exit.
  --dry-run           Print the enforce plan without X11 changes.
  --interval MS       Enforce reapply interval in milliseconds. Must be positive.

Output Options:
  --output NAME       X11 RandR output name, for example HDMI-2.
  --rate HZ           Refresh rate for mode, for example 60 or 59.94.
  --rotate DIR        Output rotation: normal, left, right, or inverted.

Window Options:
  --fullscreen        Resize and move the selected window to fill the output.

Geometry Options:
  --x N               Window X position for configure.
  --y N               Window Y position for configure.
  --width N           Window width for configure. Must be positive.
  --height N          Window height for configure. Must be positive.

Notes:
  mode requires --output and either --width with --height or --rotate.
  --rate is optional when --width and --height are provided.
  enforce requires --layout. Without --once or --dry-run, it keeps running.
  place requires WINDOW_SELECTOR, --output, and --fullscreen.
  configure requires WINDOW_SELECTOR and at least one geometry option.
  Window selector name matches are exact and must identify one mapped window.

Examples:
  xdisplay-ruler modes --output HDMI-2
  xdisplay-ruler mode --output HDMI-2 --width 1920 --height 1080 --rate 60
  xdisplay-ruler mode --output HDMI-2 --rotate left
  xdisplay-ruler raise --window-class Gnome-terminal
  xdisplay-ruler lower --window 0x800003
  xdisplay-ruler configure --window-class Gnome-terminal --x 0 --y 0
  xdisplay-ruler place --window-class Gnome-terminal --output HDMI-2 --fullscreen
  xdisplay-ruler enforce --layout layout.json --once --dry-run
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
    Enforce,
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
    layout_path: Option<String>,
    once: bool,
    dry_run: bool,
    interval_millis: usize,
    mode_width: Option<u16>,
    mode_height: Option<u16>,
    mode_refresh_millihertz: Option<u32>,
    mode_rotation: Option<OutputRotation>,
    fullscreen: bool,
    window_selector: Option<WindowSelector>,
    geometry_change: WindowGeometryChange,
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
enum WindowSelector {
    Id(WindowId),
    Title(String),
    Class(String),
    Instance(String),
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
        Command::Mode => handle_mode_command_result(run_mode_command(options), stderr),
        Command::Enforce => {
            handle_command_result(run_enforce_command(options, stdout, stderr), stderr)
        }
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

fn parse_output_rotation(value: &str) -> Result<OutputRotation, String> {
    match value {
        "normal" => Ok(OutputRotation::Normal),
        "left" => Ok(OutputRotation::Left),
        "right" => Ok(OutputRotation::Right),
        "inverted" => Ok(OutputRotation::Inverted),
        _ => Err("--rotate must be one of: normal, left, right, inverted".to_string()),
    }
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

fn run_enforce_command(
    options: CliOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let layout_path = options
        .layout_path
        .as_deref()
        .ok_or_else(|| "--layout is required".to_string())?;
    let policy = LayoutPolicy::read_from_path(layout_path).map_err(|error| error.to_string())?;
    let mut backend = build_backend(&options.backend_name)?;
    let mut state = DisplayState::new();

    if options.dry_run || options.once {
        let mode = if options.once {
            EnforcementMode::Once
        } else {
            EnforcementMode::Daemon
        };
        let plan = plan_enforcement_cycle(&mut backend, &mut state, &policy, mode)?;
        write_warnings(&plan, stderr)?;

        if options.dry_run {
            write!(stdout, "{}", enforcement_plan_report(&plan))
                .map_err(|error| error.to_string())?;
        } else {
            apply_enforcement_plan(&backend, &plan)?;
        }

        return Ok(());
    }

    loop {
        let plan =
            plan_enforcement_cycle(&mut backend, &mut state, &policy, EnforcementMode::Daemon)?;
        write_warnings(&plan, stderr)?;
        apply_enforcement_plan(&backend, &plan)?;
        thread::sleep(Duration::from_millis(options.interval_millis as u64));
    }
}

fn plan_enforcement_cycle(
    backend: &mut ConfiguredBackend,
    state: &mut DisplayState,
    policy: &LayoutPolicy,
    mode: EnforcementMode,
) -> Result<EnforcementPlan, String> {
    let events = backend
        .snapshot_events()
        .map_err(|error| error.to_string())?;
    for event in events {
        state.apply(event);
    }

    build_enforcement_plan(policy, state, mode).map_err(|error| error.to_string())
}

fn apply_enforcement_plan(
    backend: &ConfiguredBackend,
    plan: &EnforcementPlan,
) -> Result<(), String> {
    for operation in &plan.operations {
        match operation {
            LayoutOperation::ConfigureWindow { id, .. } => {
                let change = operation
                    .geometry_change()
                    .expect("configure operation should have geometry");
                backend
                    .configure_window(*id, &change)
                    .map_err(|error| error.to_string())?;
            }
            LayoutOperation::RaiseWindow { id, .. } => backend
                .raise_window(*id)
                .map_err(|error| error.to_string())?,
            LayoutOperation::StackWindowAbove { id, sibling, .. } => backend
                .stack_window_above(*id, *sibling)
                .map_err(|error| error.to_string())?,
        }
    }

    Ok(())
}

fn write_warnings(plan: &EnforcementPlan, stderr: &mut impl Write) -> Result<(), String> {
    for warning in &plan.warnings {
        writeln!(stderr, "warning: {warning}").map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn enforcement_plan_report(plan: &EnforcementPlan) -> String {
    let mut report = format!(
        "xdisplay-ruler enforce dry-run\noperations: {}\n",
        plan.operations.len()
    );

    for operation in &plan.operations {
        report.push_str(&format!("- {operation}\n"));
    }

    report
}

fn run_stack_command(options: CliOptions, command: StackCommand) -> Result<(), String> {
    let selector = required_window_selector(&options)?;
    let backend = build_backend(&options.backend_name)?;
    let window_id = resolve_window_selector(&backend, &selector)?;

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

fn run_mode_command(options: CliOptions) -> Result<OutputModeChange, String> {
    let output_name = options
        .output_name
        .ok_or_else(|| "--output is required".to_string())?;
    if options.mode_width.is_some() != options.mode_height.is_some() {
        return Err("--width and --height must be provided together".to_string());
    }
    if options.mode_width.is_none() && options.mode_rotation.is_none() {
        return Err("--width with --height or --rotate is required".to_string());
    }
    if options.mode_refresh_millihertz.is_some() && options.mode_width.is_none() {
        return Err("--rate requires --width and --height".to_string());
    }

    let selection = OutputModeSelection {
        width: options.mode_width,
        height: options.mode_height,
        refresh_millihertz: options.mode_refresh_millihertz,
        rotation: options.mode_rotation,
    };
    let backend = build_backend(&options.backend_name)?;

    backend
        .set_output_mode(&output_name, &selection)
        .map_err(|error| error.to_string())
}

fn run_place_command(options: CliOptions) -> Result<(), String> {
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

fn run_configure_command(options: CliOptions) -> Result<(), String> {
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

fn resolve_window_from_list(
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
            escape_report_value(window.title.as_deref().unwrap_or("")),
            escape_report_value(window.class_name.as_deref().unwrap_or("")),
            escape_report_value(window.instance_name.as_deref().unwrap_or(""))
        ));
    }

    message
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

fn handle_mode_command_result(
    result: Result<OutputModeChange, String>,
    stderr: &mut impl Write,
) -> io::Result<CliExit> {
    match result {
        Ok(change) => {
            for warning in change.warnings {
                writeln!(stderr, "warning: {warning}")?;
            }
            Ok(CliExit::Success)
        }
        Err(message) => {
            writeln!(stderr, "{message}")?;
            writeln!(stderr, "try --help")?;
            Ok(CliExit::UsageError)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{handle_mode_command_result, run, CliExit, WindowSelector};
    use crate::{OutputMode, Rect, WindowId, WindowInfo};
    use crate::{OutputModeChange, OutputRotation};

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
    fn requires_layout_for_enforce() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["enforce", "--once"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--layout is required\ntry --help\n"
        );
    }

    #[test]
    fn validates_enforce_interval() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            ["enforce", "--layout", "layout.json", "--interval", "0"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--interval must be a positive integer\ntry --help\n"
        );
    }

    #[test]
    fn dry_run_enforce_exits_after_printing_plan() {
        let layout_path = write_temp_layout(
            r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" }
                ]
            }"#,
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            [
                "enforce",
                "--backend",
                "in-memory",
                "--layout",
                layout_path.to_str().expect("temp path should be UTF-8"),
                "--dry-run",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::Success);
        assert_eq!(
            String::from_utf8_lossy(&stdout),
            "xdisplay-ruler enforce dry-run\noperations: 0\n"
        );
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "warning: window not found: app_id:\"Player\"\n"
        );

        fs::remove_file(layout_path).expect("temp layout should be removable");
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
    fn requires_output_and_mode_or_rotation_for_mode_command() {
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
            "--width with --height or --rotate is required\ntry --help\n"
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
            "--width and --height must be provided together\ntry --help\n"
        );

        stderr.clear();
        let exit = run(
            ["mode", "--output", "HDMI-2", "--rate", "60"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--width with --height or --rotate is required\ntry --help\n"
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

        stderr.clear();
        let exit = run(
            ["mode", "--output", "HDMI-2", "--rotate", "sideways"],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "--rotate must be one of: normal, left, right, inverted\ntry --help\n"
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
                "--rotate",
                "left",
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
    fn mode_command_warnings_keep_success_exit_status() {
        let mut stderr = Vec::new();
        let exit = handle_mode_command_result(
            Ok(OutputModeChange {
                warnings: vec![
                    "output mode changed, but touch remapping failed: test failure".to_string(),
                ],
            }),
            &mut stderr,
        )
        .expect("mode result should be handled");

        assert_eq!(exit, CliExit::Success);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "warning: output mode changed, but touch remapping failed: test failure\n"
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
    fn parses_output_rotations() {
        assert_eq!(
            super::parse_output_rotation("normal"),
            Ok(OutputRotation::Normal)
        );
        assert_eq!(
            super::parse_output_rotation("left"),
            Ok(OutputRotation::Left)
        );
        assert_eq!(
            super::parse_output_rotation("right"),
            Ok(OutputRotation::Right)
        );
        assert_eq!(
            super::parse_output_rotation("inverted"),
            Ok(OutputRotation::Inverted)
        );
        assert!(super::parse_output_rotation("sideways").is_err());
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
            "--window, --window-title, --window-class, or --window-instance is required\ntry --help\n"
        );
    }

    #[test]
    fn rejects_multiple_window_selectors() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(
            [
                "raise",
                "--window",
                "0x800003",
                "--window-class",
                "Gnome-terminal",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            concat!(
                "--window, --window-title, --window-class, and --window-instance are mutually exclusive\n",
                "try --help\n",
            )
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
            "--window, --window-title, --window-class, or --window-instance is required\ntry --help\n"
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

    #[test]
    fn resolves_windows_by_exact_title_class_or_instance() {
        let windows = selector_test_windows();

        assert_eq!(
            super::resolve_window_from_list(
                &windows,
                &WindowSelector::Title("Terminal".to_string()),
                "Terminal",
            ),
            Ok(WindowId(0x10))
        );
        assert_eq!(
            super::resolve_window_from_list(
                &windows,
                &WindowSelector::Class("Code".to_string()),
                "Code",
            ),
            Ok(WindowId(0x20))
        );
        assert_eq!(
            super::resolve_window_from_list(
                &windows,
                &WindowSelector::Instance("code".to_string()),
                "code",
            ),
            Ok(WindowId(0x20))
        );
    }

    #[test]
    fn reports_missing_or_ambiguous_window_selectors() {
        let windows = selector_test_windows();

        assert_eq!(
            super::resolve_window_from_list(
                &windows,
                &WindowSelector::Class("Missing".to_string()),
                "Missing",
            ),
            Err("window not found: Missing".to_string())
        );

        let error = super::resolve_window_from_list(
            &windows,
            &WindowSelector::Class("Firefox".to_string()),
            "Firefox",
        )
        .expect_err("selector should be ambiguous");

        assert!(error.contains("window selector is ambiguous: Firefox"));
        assert!(error.contains("0x30"));
        assert!(error.contains("0x40"));
    }

    fn selector_test_windows() -> Vec<WindowInfo> {
        let mut terminal = WindowInfo::mapped(WindowId(0x10), Rect::new(0, 0, 800, 600));
        terminal.title = Some("Terminal".to_string());
        terminal.class_name = Some("Gnome-terminal".to_string());
        terminal.instance_name = Some("gnome-terminal-server".to_string());

        let mut code = WindowInfo::mapped(WindowId(0x20), Rect::new(0, 0, 800, 600));
        code.title = Some("main.rs".to_string());
        code.class_name = Some("Code".to_string());
        code.instance_name = Some("code".to_string());

        let mut firefox = WindowInfo::mapped(WindowId(0x30), Rect::new(0, 0, 800, 600));
        firefox.title = Some("Docs".to_string());
        firefox.class_name = Some("Firefox".to_string());
        firefox.instance_name = Some("firefox".to_string());

        let mut second_firefox = WindowInfo::mapped(WindowId(0x40), Rect::new(0, 0, 800, 600));
        second_firefox.title = Some("Mail".to_string());
        second_firefox.class_name = Some("Firefox".to_string());
        second_firefox.instance_name = Some("firefox".to_string());

        vec![terminal, code, firefox, second_firefox]
    }

    fn write_temp_layout(content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after UNIX epoch")
            .as_nanos();
        path.push(format!(
            "xdisplay-ruler-test-{}-{unique}.json",
            std::process::id()
        ));
        fs::write(&path, content).expect("temp layout should be writable");
        path
    }
}
