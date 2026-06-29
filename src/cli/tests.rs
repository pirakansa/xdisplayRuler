use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{OutputMode, OutputModeChange, OutputRotation, Rect, WindowId, WindowInfo};

use super::{
    command::{handle_mode_command_result, resolve_window_from_list},
    options::{parse_output_rotation, parse_refresh_millihertz, parse_window_id, WindowSelector},
    report::modes_report,
    run, CliExit,
};

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

    let exit = run(["--backend", "unsupported"], &mut stdout, &mut stderr).expect("cli should run");

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
    assert_eq!(parse_refresh_millihertz("60"), Ok(60_000));
    assert_eq!(parse_refresh_millihertz("59.94"), Ok(59_940));
    assert_eq!(parse_refresh_millihertz("59.940"), Ok(59_940));
    assert!(parse_refresh_millihertz("0").is_err());
    assert!(parse_refresh_millihertz("59.9400").is_err());
    assert!(parse_refresh_millihertz("fast").is_err());
}

#[test]
fn parses_output_rotations() {
    assert_eq!(parse_output_rotation("normal"), Ok(OutputRotation::Normal));
    assert_eq!(parse_output_rotation("left"), Ok(OutputRotation::Left));
    assert_eq!(parse_output_rotation("right"), Ok(OutputRotation::Right));
    assert_eq!(
        parse_output_rotation("inverted"),
        Ok(OutputRotation::Inverted)
    );
    assert!(parse_output_rotation("sideways").is_err());
}

#[test]
fn renders_modes_report() {
    let report = modes_report(
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

    let exit =
        run(["place", "--window", "0x800003"], &mut stdout, &mut stderr).expect("cli should run");

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

    let exit = run(["configure", "--x", "10"], &mut stdout, &mut stderr).expect("cli should run");

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
    assert_eq!(parse_window_id("0x800003"), Ok(crate::WindowId(0x800003)));
    assert_eq!(parse_window_id("8388611"), Ok(crate::WindowId(0x800003)));
    assert!(parse_window_id("not-a-window").is_err());
}

#[test]
fn resolves_windows_by_exact_title_class_or_instance() {
    let windows = selector_test_windows();

    assert_eq!(
        resolve_window_from_list(
            &windows,
            &WindowSelector::Title("Terminal".to_string()),
            "Terminal",
        ),
        Ok(WindowId(0x10))
    );
    assert_eq!(
        resolve_window_from_list(&windows, &WindowSelector::Class("Code".to_string()), "Code"),
        Ok(WindowId(0x20))
    );
    assert_eq!(
        resolve_window_from_list(
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
        resolve_window_from_list(
            &windows,
            &WindowSelector::Class("Missing".to_string()),
            "Missing",
        ),
        Err("window not found: Missing".to_string())
    );

    let error = resolve_window_from_list(
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
