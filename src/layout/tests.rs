use crate::{DisplayEvent, DisplayOutput, DisplayState, Rect, WindowId, WindowInfo};

use super::{
    build_enforcement_plan, EnforcementMode, LayoutError, LayoutOperation, LayoutPolicy,
    UnmanagedWindowsPolicy, WindowSelector,
};

#[test]
fn parses_minimal_layout_and_defaults_unmanaged_windows() {
    let layout = LayoutPolicy::from_json_str(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" }
                ]
            }"#,
    )
    .expect("layout should parse");

    assert_eq!(layout.unmanaged_windows, UnmanagedWindowsPolicy::AllowAbove);
    assert_eq!(layout.windows.len(), 1);
    assert_eq!(
        layout.windows[0].selector,
        WindowSelector::AppId("Player".to_string())
    );
}

#[test]
fn parses_class_and_instance_selectors() {
    let layout = LayoutPolicy::from_json_str(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "class": "Player" }, "output": "HDMI-2" },
                    { "selector": { "instance": "overlay" }, "output": "HDMI-2" }
                ]
            }"#,
    )
    .expect("layout should parse");

    assert_eq!(
        layout.windows[0].selector,
        WindowSelector::Class("Player".to_string())
    );
    assert_eq!(
        layout.windows[1].selector,
        WindowSelector::Instance("overlay".to_string())
    );
}

#[test]
fn rejects_unknown_fields_and_unsupported_schema_version() {
    assert!(LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[],"placement":"fullscreen"}"#
    )
    .is_err());

    assert!(matches!(
        LayoutPolicy::from_json_str(r#"{"schema_version":2,"windows":[]}"#),
        Err(LayoutError::UnsupportedSchemaVersion(2))
    ));
}

#[test]
fn selector_must_contain_exactly_one_supported_field() {
    assert!(LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[{"selector":{},"output":"HDMI-2"}]}"#
    )
    .is_err());
    assert!(
        LayoutPolicy::from_json_str(
            r#"{"schema_version":1,"windows":[{"selector":{"title":"A","app_id":"B"},"output":"HDMI-2"}]}"#
        )
        .is_err()
    );
    assert!(
        LayoutPolicy::from_json_str(
            r#"{"schema_version":1,"windows":[{"selector":{"class":"A","instance":"B"},"output":"HDMI-2"}]}"#
        )
        .is_err()
    );
}

#[test]
fn matches_id_title_app_id_class_and_instance_selectors() {
    let mut state = test_state();
    state.apply(DisplayEvent::WindowConfigured {
        id: WindowId(0x20),
        geometry: Rect::new(0, 0, 800, 600),
    });
    let layout = LayoutPolicy::from_json_str(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "id": "0x10" }, "output": "HDMI-2" },
                    { "selector": { "title": "Overlay" }, "output": "HDMI-2" },
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" },
                    { "selector": { "class": "Player" }, "output": "HDMI-2" },
                    { "selector": { "instance": "overlay" }, "output": "HDMI-2" }
                ]
            }"#,
    )
    .expect("layout should parse");

    let plan =
        build_enforcement_plan(&layout, &state, EnforcementMode::Once).expect("plan should build");

    assert_eq!(
        plan.operations
            .iter()
            .filter(|operation| matches!(operation, LayoutOperation::ConfigureWindow { .. }))
            .count(),
        5
    );
}

#[test]
fn reports_missing_and_ambiguous_selectors_in_once_mode() {
    let state = test_state();
    let missing = LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[{"selector":{"app_id":"Missing"},"output":"HDMI-2"}]}"#,
    )
    .expect("layout should parse");

    assert!(matches!(
        build_enforcement_plan(&missing, &state, EnforcementMode::Once),
        Err(LayoutError::SelectorNotFound(WindowSelector::AppId(_)))
    ));

    let ambiguous = LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[{"selector":{"app_id":"Firefox"},"output":"HDMI-2"}]}"#,
    )
    .expect("layout should parse");

    assert!(matches!(
        build_enforcement_plan(&ambiguous, &state, EnforcementMode::Once),
        Err(LayoutError::SelectorAmbiguous { .. })
    ));
}

#[test]
fn daemon_mode_warns_and_skips_unresolved_rules() {
    let state = test_state();
    let layout = LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[{"selector":{"app_id":"Missing"},"output":"Missing"}]}"#,
    )
    .expect("layout should parse");

    let plan = build_enforcement_plan(&layout, &state, EnforcementMode::Daemon)
        .expect("daemon plan should be recoverable");

    assert!(plan.operations.is_empty());
    assert_eq!(plan.warnings, vec!["window not found: app_id:\"Missing\""]);
}

#[test]
fn plans_fit_to_output_geometry_only_when_geometry_differs() {
    let state = test_state();
    let layout = LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[{"selector":{"app_id":"Player"},"output":"HDMI-2"}]}"#,
    )
    .expect("layout should parse");

    let plan =
        build_enforcement_plan(&layout, &state, EnforcementMode::Once).expect("plan should build");

    assert_eq!(
        plan.operations,
        vec![LayoutOperation::ConfigureWindow {
            id: WindowId(0x10),
            selector: WindowSelector::AppId("Player".to_string()),
            output: "HDMI-2".to_string(),
            geometry: Rect::new(100, 50, 1920, 1080),
        }]
    );
}

#[test]
fn keep_below_managed_plans_raises_in_layout_order() {
    let state = test_state();
    let layout = LayoutPolicy::from_json_str(
        r#"{
                "schema_version": 1,
                "unmanaged_windows": "keep_below_managed",
                "windows": [
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" },
                    { "selector": { "title": "Overlay" }, "output": "HDMI-2" }
                ]
            }"#,
    )
    .expect("layout should parse");

    let plan =
        build_enforcement_plan(&layout, &state, EnforcementMode::Once).expect("plan should build");

    assert!(matches!(
        plan.operations[1],
        LayoutOperation::RaiseWindow {
            id: WindowId(0x10),
            ..
        }
    ));
    assert!(matches!(
        plan.operations[2],
        LayoutOperation::RaiseWindow {
            id: WindowId(0x20),
            ..
        }
    ));
}

#[test]
fn allow_above_plans_relative_stack_operations_for_managed_windows_only() {
    let mut state = test_state();
    state.apply(DisplayEvent::WindowRaised(WindowId(0x10)));
    let layout = LayoutPolicy::from_json_str(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" },
                    { "selector": { "title": "Overlay" }, "output": "HDMI-2" }
                ]
            }"#,
    )
    .expect("layout should parse");

    let plan =
        build_enforcement_plan(&layout, &state, EnforcementMode::Once).expect("plan should build");

    assert!(plan.operations.iter().any(|operation| matches!(
        operation,
        LayoutOperation::StackWindowAbove {
            id: WindowId(0x20),
            sibling: WindowId(0x10),
            ..
        }
    )));
    assert!(!plan
        .operations
        .iter()
        .any(|operation| matches!(operation, LayoutOperation::RaiseWindow { .. })));
}

#[test]
fn allow_above_skips_stack_operations_when_managed_order_already_matches() {
    let state = test_state();
    let layout = LayoutPolicy::from_json_str(
        r#"{
                "schema_version": 1,
                "windows": [
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" },
                    { "selector": { "title": "Overlay" }, "output": "HDMI-2" }
                ]
            }"#,
    )
    .expect("layout should parse");

    let plan =
        build_enforcement_plan(&layout, &state, EnforcementMode::Once).expect("plan should build");

    assert!(!plan.operations.iter().any(|operation| {
        matches!(operation, LayoutOperation::StackWindowAbove { .. })
            || matches!(operation, LayoutOperation::RaiseWindow { .. })
    }));
}

#[test]
fn reports_missing_or_disconnected_outputs() {
    let mut state = test_state();
    state.apply(DisplayEvent::OutputDisconnected {
        name: "HDMI-2".to_string(),
    });
    let layout = LayoutPolicy::from_json_str(
        r#"{"schema_version":1,"windows":[{"selector":{"app_id":"Player"},"output":"HDMI-2"}]}"#,
    )
    .expect("layout should parse");

    assert!(matches!(
        build_enforcement_plan(&layout, &state, EnforcementMode::Once),
        Err(LayoutError::OutputDisconnected(_))
    ));
}

fn test_state() -> DisplayState {
    let mut state = DisplayState::new();
    state.apply(DisplayEvent::OutputConnected(DisplayOutput::connected(
        "HDMI-2",
        Rect::new(100, 50, 1920, 1080),
        true,
    )));

    let mut player = WindowInfo::mapped(WindowId(0x10), Rect::new(0, 0, 800, 600));
    player.title = Some("Player".to_string());
    player.class_name = Some("Player".to_string());
    player.instance_name = Some("player".to_string());
    state.apply(DisplayEvent::WindowMapped(player));

    let mut overlay = WindowInfo::mapped(WindowId(0x20), Rect::new(100, 50, 1920, 1080));
    overlay.title = Some("Overlay".to_string());
    overlay.class_name = Some("Overlay".to_string());
    overlay.instance_name = Some("overlay".to_string());
    state.apply(DisplayEvent::WindowMapped(overlay));

    let mut firefox = WindowInfo::mapped(WindowId(0x30), Rect::new(0, 0, 800, 600));
    firefox.class_name = Some("Firefox".to_string());
    firefox.instance_name = Some("firefox".to_string());
    state.apply(DisplayEvent::WindowMapped(firefox));

    let mut second_firefox = WindowInfo::mapped(WindowId(0x40), Rect::new(0, 0, 800, 600));
    second_firefox.class_name = Some("Firefox".to_string());
    second_firefox.instance_name = Some("firefox".to_string());
    state.apply(DisplayEvent::WindowMapped(second_firefox));

    state
}
