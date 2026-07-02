use std::fmt;

use crate::{report::escape_value, DisplayState, Rect, WindowGeometryChange, WindowId, WindowInfo};

use super::policy::{
    LayoutError, LayoutPolicy, ManagedWindowRule, UnmanagedWindowsPolicy, WindowSelector,
};

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
    ActivateWindow {
        id: WindowId,
        selector: WindowSelector,
    },
    StackWindowAbove {
        id: WindowId,
        selector: WindowSelector,
        sibling: WindowId,
    },
}

pub fn build_enforcement_plan(
    policy: &LayoutPolicy,
    state: &DisplayState,
    mode: EnforcementMode,
) -> Result<EnforcementPlan, LayoutError> {
    let mut plan = EnforcementPlan::default();
    let mut managed_windows = Vec::new();
    let mut active_window = None;

    for rule in &policy.windows {
        let Some(window) = resolve_window_rule(rule, state, mode, &mut plan.warnings)? else {
            continue;
        };
        let Some(output_geometry) = resolve_output_geometry(rule, state, mode, &mut plan.warnings)?
        else {
            continue;
        };

        managed_windows.push((window.id, rule.selector.clone()));
        if rule.activate {
            active_window = Some((window.id, rule.selector.clone()));
        }

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

    if let Some((id, selector)) = active_window {
        plan.operations
            .push(LayoutOperation::ActivateWindow { id, selector });
    }

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
            Self::RaiseWindow { .. }
            | Self::ActivateWindow { .. }
            | Self::StackWindowAbove { .. } => None,
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
                escape_value(output)
            ),
            Self::RaiseWindow { id, selector } => {
                write!(formatter, "raise {id} selector={selector}")
            }
            Self::ActivateWindow { id, selector } => {
                write!(formatter, "activate {id} selector={selector}")
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
        WindowSelector::Class(class_name) => {
            window.class_name.as_deref() == Some(class_name.as_str())
        }
        WindowSelector::Instance(instance_name) => {
            window.instance_name.as_deref() == Some(instance_name.as_str())
        }
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
