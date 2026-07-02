use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{Rect, WindowId, WindowInfo};

use super::{
    command::resolve_window_from_list,
    options::{parse_options, parse_window_id, Command, WindowSelector},
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
                    { "selector": { "class": "Player" }, "output": "HDMI-2" }
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
        "warning: window not found: class:\"Player\"\n"
    );

    fs::remove_file(layout_path).expect("temp layout should be removable");
}

#[test]
fn once_enforce_warns_and_skips_unresolved_rules() {
    let layout_path = write_temp_layout(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "class": "Player" }, "output": "HDMI-2" }
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
            "--once",
        ],
        &mut stdout,
        &mut stderr,
    )
    .expect("cli should run");

    assert_eq!(exit, CliExit::Success);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&stderr),
        "warning: window not found: class:\"Player\"\n"
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
fn rejects_removed_mode_commands() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run(["modes"], &mut stdout, &mut stderr).expect("cli should run");

    assert_eq!(exit, CliExit::UsageError);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&stderr),
        "unknown command: modes\ntry --help\n"
    );

    stderr.clear();
    let exit = run(["mode"], &mut stdout, &mut stderr).expect("cli should run");

    assert_eq!(exit, CliExit::UsageError);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&stderr),
        "unknown command: mode\ntry --help\n"
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
fn parses_activate_command_with_x11_default_backend() {
    let options = parse_options(&[
        "activate".to_string(),
        "--window".to_string(),
        "0x800003".to_string(),
    ])
    .expect("activate options should parse");

    assert_eq!(options.command, Command::Activate);
    assert_eq!(options.backend_name, "x11");
    assert_eq!(
        options.window_selector,
        Some(WindowSelector::Id(WindowId(0x800003)))
    );
}

#[test]
fn requires_window_for_activate() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run(["activate"], &mut stdout, &mut stderr).expect("cli should run");

    assert_eq!(exit, CliExit::UsageError);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&stderr),
        "--window, --window-title, --window-class, or --window-instance is required\ntry --help\n"
    );
}

#[test]
fn rejects_activate_for_in_memory_backend() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run(
        ["activate", "--backend", "in-memory", "--window", "0x800003"],
        &mut stdout,
        &mut stderr,
    )
    .expect("cli should run");

    assert_eq!(exit, CliExit::UsageError);
    assert!(stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&stderr),
        "in-memory backend cannot activate X11 windows\ntry --help\n"
    );
}

#[test]
fn help_lists_activate_command() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = run(["--help"], &mut stdout, &mut stderr).expect("cli should run");

    assert_eq!(exit, CliExit::Success);
    assert!(stderr.is_empty());
    let help = String::from_utf8_lossy(&stdout);
    assert!(help.contains("xdisplay-ruler activate WINDOW_SELECTOR [--backend x11]"));
    assert!(help.contains("activate  Set input focus to a window."));
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
