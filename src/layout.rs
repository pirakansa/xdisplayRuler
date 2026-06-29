use std::{fmt, fs, path::Path};

use serde::Deserialize;

use crate::{DisplayState, Rect, WindowGeometryChange, WindowId, WindowInfo};

const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutPolicy {
    pub schema_version: u32,
    #[serde(default)]
    pub unmanaged_windows: UnmanagedWindowsPolicy,
    pub windows: Vec<ManagedWindowRule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedWindowRule {
    pub selector: WindowSelector,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowSelector {
    Id(WindowId),
    Title(String),
    AppId(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnmanagedWindowsPolicy {
    #[default]
    AllowAbove,
    KeepBelowManaged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnforcementMode {
    Once,
    Daemon,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnforcementPlan {
    pub operations: Vec<LayoutOperation>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutOperation {
    ConfigureWindow {
        id: WindowId,
        selector: WindowSelector,
        output: String,
        geometry: Rect,
    },
    RaiseWindow {
        id: WindowId,
        selector: WindowSelector,
    },
    StackWindowAbove {
        id: WindowId,
        selector: WindowSelector,
        sibling: WindowId,
    },
}

impl LayoutPolicy {
    pub fn read_from_path(path: impl AsRef<Path>) -> Result<Self, LayoutError> {
        let content = fs::read_to_string(path).map_err(LayoutError::Read)?;
        Self::from_json_str(&content)
    }

    pub fn from_json_str(content: &str) -> Result<Self, LayoutError> {
        let policy = serde_json::from_str::<Self>(content).map_err(LayoutError::Json)?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(LayoutError::UnsupportedSchemaVersion(self.schema_version));
        }

        Ok(())
    }
}

pub fn build_enforcement_plan(
    policy: &LayoutPolicy,
    state: &DisplayState,
    mode: EnforcementMode,
) -> Result<EnforcementPlan, LayoutError> {
    let mut plan = EnforcementPlan::default();
    let mut managed_windows = Vec::new();

    for rule in &policy.windows {
        let Some(window) = resolve_window_rule(rule, state, mode, &mut plan.warnings)? else {
            continue;
        };
        let Some(output_geometry) = resolve_output_geometry(rule, state, mode, &mut plan.warnings)?
        else {
            continue;
        };

        managed_windows.push((window.id, rule.selector.clone()));

        if window.geometry != output_geometry {
            plan.operations.push(LayoutOperation::ConfigureWindow {
                id: window.id,
                selector: rule.selector.clone(),
                output: rule.output.clone(),
                geometry: output_geometry,
            });
        }
    }

    plan.operations.extend(stack_policy_operations(
        policy.unmanaged_windows,
        state,
        &managed_windows,
    ));

    Ok(plan)
}

impl LayoutOperation {
    pub fn geometry_change(&self) -> Option<WindowGeometryChange> {
        match self {
            Self::ConfigureWindow { geometry, .. } => Some(WindowGeometryChange {
                x: Some(geometry.x),
                y: Some(geometry.y),
                width: Some(geometry.width),
                height: Some(geometry.height),
            }),
            Self::RaiseWindow { .. } | Self::StackWindowAbove { .. } => None,
        }
    }
}

impl fmt::Display for LayoutOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigureWindow {
                id,
                selector,
                output,
                geometry,
            } => write!(
                formatter,
                "configure {id} selector={selector} output=\"{}\" geometry={geometry}",
                escape_report_value(output)
            ),
            Self::RaiseWindow { id, selector } => {
                write!(formatter, "raise {id} selector={selector}")
            }
            Self::StackWindowAbove {
                id,
                selector,
                sibling,
            } => write!(
                formatter,
                "stack-above {id} selector={selector} sibling={sibling}"
            ),
        }
    }
}

impl fmt::Display for WindowSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(formatter, "id:{id}"),
            Self::Title(title) => write!(formatter, "title:\"{}\"", escape_report_value(title)),
            Self::AppId(app_id) => write!(formatter, "app_id:\"{}\"", escape_report_value(app_id)),
        }
    }
}

impl<'de> Deserialize<'de> for WindowSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SelectorFields {
            id: Option<String>,
            title: Option<String>,
            app_id: Option<String>,
        }

        let fields = SelectorFields::deserialize(deserializer)?;
        let set_count = usize::from(fields.id.is_some())
            + usize::from(fields.title.is_some())
            + usize::from(fields.app_id.is_some());

        if set_count != 1 {
            return Err(serde::de::Error::custom(
                "selector must contain exactly one of: id, title, app_id",
            ));
        }

        if let Some(id) = fields.id {
            return parse_window_id(&id)
                .map(Self::Id)
                .map_err(serde::de::Error::custom);
        }
        if let Some(title) = fields.title {
            return Ok(Self::Title(title));
        }
        if let Some(app_id) = fields.app_id {
            return Ok(Self::AppId(app_id));
        }

        unreachable!("selector count was checked above")
    }
}

#[derive(Debug)]
pub enum LayoutError {
    Read(std::io::Error),
    Json(serde_json::Error),
    UnsupportedSchemaVersion(u32),
    SelectorNotFound(WindowSelector),
    SelectorAmbiguous {
        selector: WindowSelector,
        matches: Vec<WindowId>,
    },
    OutputNotFound(String),
    OutputDisconnected(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "layout read failed: {error}"),
            Self::Json(error) => write!(formatter, "layout JSON is invalid: {error}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported layout schema_version: {version}")
            }
            Self::SelectorNotFound(selector) => write!(formatter, "window not found: {selector}"),
            Self::SelectorAmbiguous { selector, matches } => {
                write!(formatter, "window selector is ambiguous: {selector}")?;
                for id in matches {
                    write!(formatter, "\n- {id}")?;
                }
                Ok(())
            }
            Self::OutputNotFound(output) => write!(formatter, "output not found: {output}"),
            Self::OutputDisconnected(output) => {
                write!(formatter, "output is disconnected: {output}")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

fn resolve_window_rule<'a>(
    rule: &ManagedWindowRule,
    state: &'a DisplayState,
    mode: EnforcementMode,
    warnings: &mut Vec<String>,
) -> Result<Option<&'a WindowInfo>, LayoutError> {
    let matches = state
        .windows()
        .iter()
        .filter(|window| window.mapped && selector_matches(window, &rule.selector))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => recoverable_or_error(
            mode,
            LayoutError::SelectorNotFound(rule.selector.clone()),
            warnings,
        ),
        [window] => Ok(Some(*window)),
        _ => {
            let error = LayoutError::SelectorAmbiguous {
                selector: rule.selector.clone(),
                matches: matches.iter().map(|window| window.id).collect(),
            };
            recoverable_or_error(mode, error, warnings)
        }
    }
}

fn resolve_output_geometry(
    rule: &ManagedWindowRule,
    state: &DisplayState,
    mode: EnforcementMode,
    warnings: &mut Vec<String>,
) -> Result<Option<Rect>, LayoutError> {
    let Some(output) = state
        .outputs()
        .iter()
        .find(|output| output.name == rule.output)
    else {
        return recoverable_or_error(
            mode,
            LayoutError::OutputNotFound(rule.output.clone()),
            warnings,
        );
    };

    if !output.connected {
        return recoverable_or_error(
            mode,
            LayoutError::OutputDisconnected(rule.output.clone()),
            warnings,
        );
    }

    Ok(Some(output.geometry.clone()))
}

fn recoverable_or_error<T>(
    mode: EnforcementMode,
    error: LayoutError,
    warnings: &mut Vec<String>,
) -> Result<Option<T>, LayoutError> {
    match mode {
        EnforcementMode::Once => Err(error),
        EnforcementMode::Daemon => {
            warnings.push(error.to_string());
            Ok(None)
        }
    }
}

fn selector_matches(window: &WindowInfo, selector: &WindowSelector) -> bool {
    match selector {
        WindowSelector::Id(id) => window.id == *id,
        WindowSelector::Title(title) => window.title.as_deref() == Some(title.as_str()),
        WindowSelector::AppId(app_id) => window.class_name.as_deref() == Some(app_id.as_str()),
    }
}

fn managed_order_matches(
    state: &DisplayState,
    managed_windows: &[(WindowId, WindowSelector)],
) -> bool {
    let desired = managed_windows
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let actual = state
        .stacking_order()
        .iter()
        .copied()
        .filter(|id| desired.contains(id))
        .collect::<Vec<_>>();

    actual == desired
}

fn stack_policy_operations(
    policy: UnmanagedWindowsPolicy,
    state: &DisplayState,
    managed_windows: &[(WindowId, WindowSelector)],
) -> Vec<LayoutOperation> {
    match policy {
        UnmanagedWindowsPolicy::AllowAbove => allow_above_stack_operations(state, managed_windows),
        UnmanagedWindowsPolicy::KeepBelowManaged => managed_windows
            .iter()
            .map(|(id, selector)| LayoutOperation::RaiseWindow {
                id: *id,
                selector: selector.clone(),
            })
            .collect(),
    }
}

fn allow_above_stack_operations(
    state: &DisplayState,
    managed_windows: &[(WindowId, WindowSelector)],
) -> Vec<LayoutOperation> {
    if managed_order_matches(state, managed_windows) {
        return Vec::new();
    }

    managed_windows
        .windows(2)
        .map(|pair| {
            let [(sibling, _), (id, selector)] = pair else {
                unreachable!("windows(2) returns exactly two items")
            };
            LayoutOperation::StackWindowAbove {
                id: *id,
                selector: selector.clone(),
                sibling: *sibling,
            }
        })
        .collect()
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
        .map_err(|_| format!("id must be an X11 window id, got: {value}"))?;

    Ok(WindowId(parsed))
}

fn escape_report_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
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
    }

    #[test]
    fn matches_id_title_and_app_id_selectors() {
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
                    { "selector": { "app_id": "Player" }, "output": "HDMI-2" }
                ]
            }"#,
        )
        .expect("layout should parse");

        let plan = build_enforcement_plan(&layout, &state, EnforcementMode::Once)
            .expect("plan should build");

        assert_eq!(
            plan.operations
                .iter()
                .filter(|operation| matches!(operation, LayoutOperation::ConfigureWindow { .. }))
                .count(),
            3
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

        let plan = build_enforcement_plan(&layout, &state, EnforcementMode::Once)
            .expect("plan should build");

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

        let plan = build_enforcement_plan(&layout, &state, EnforcementMode::Once)
            .expect("plan should build");

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

        let plan = build_enforcement_plan(&layout, &state, EnforcementMode::Once)
            .expect("plan should build");

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

        let plan = build_enforcement_plan(&layout, &state, EnforcementMode::Once)
            .expect("plan should build");

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
        state.apply(DisplayEvent::WindowMapped(player));

        let mut overlay = WindowInfo::mapped(WindowId(0x20), Rect::new(100, 50, 1920, 1080));
        overlay.title = Some("Overlay".to_string());
        overlay.class_name = Some("Overlay".to_string());
        state.apply(DisplayEvent::WindowMapped(overlay));

        let mut firefox = WindowInfo::mapped(WindowId(0x30), Rect::new(0, 0, 800, 600));
        firefox.class_name = Some("Firefox".to_string());
        state.apply(DisplayEvent::WindowMapped(firefox));

        let mut second_firefox = WindowInfo::mapped(WindowId(0x40), Rect::new(0, 0, 800, 600));
        second_firefox.class_name = Some("Firefox".to_string());
        state.apply(DisplayEvent::WindowMapped(second_firefox));

        state
    }
}
