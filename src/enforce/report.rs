use std::{collections::HashSet, io::Write};

use crate::EnforcementPlan;

pub(super) fn write_warnings(
    plan: &EnforcementPlan,
    stderr: &mut impl Write,
) -> Result<(), String> {
    for warning in &plan.warnings {
        writeln!(stderr, "warning: {warning}").map_err(|error| error.to_string())?;
    }

    Ok(())
}

pub(super) fn write_new_warnings(
    plan: &EnforcementPlan,
    stderr: &mut impl Write,
    previous_warnings: &mut HashSet<String>,
) -> Result<(), String> {
    let current_warnings = plan.warnings.iter().cloned().collect::<HashSet<_>>();

    for warning in &plan.warnings {
        if !previous_warnings.contains(warning) {
            writeln!(stderr, "warning: {warning}").map_err(|error| error.to_string())?;
        }
    }

    *previous_warnings = current_warnings;
    Ok(())
}

pub(super) fn write_dry_run_report(
    plan: &EnforcementPlan,
    stdout: &mut impl Write,
) -> Result<(), String> {
    write!(stdout, "{}", plan_report(plan)).map_err(|error| error.to_string())
}

fn plan_report(plan: &EnforcementPlan) -> String {
    let mut report = format!(
        "xdisplay-ruler enforce dry-run\noperations: {}\n",
        plan.operations.len()
    );

    for operation in &plan.operations {
        report.push_str(&format!("- {operation}\n"));
    }

    report
}

#[cfg(test)]
mod tests {
    use crate::{layout::WindowSelector, EnforcementPlan, LayoutOperation, Rect, WindowId};

    use std::collections::HashSet;

    use super::{write_dry_run_report, write_new_warnings};

    #[test]
    fn dry_run_report_includes_planned_operations() {
        let plan = EnforcementPlan {
            operations: vec![LayoutOperation::ConfigureWindow {
                id: WindowId(0x20),
                selector: WindowSelector::AppId("Player".to_string()),
                output: "HDMI-2".to_string(),
                geometry: Rect::new(0, 0, 1920, 1080),
            }],
            warnings: Vec::new(),
        };
        let mut stdout = Vec::new();

        write_dry_run_report(&plan, &mut stdout).expect("dry-run report should render");

        assert_eq!(
            String::from_utf8_lossy(&stdout),
            concat!(
                "xdisplay-ruler enforce dry-run\n",
                "operations: 1\n",
                "- configure 0x20 selector=app_id:\"Player\" ",
                "output=\"HDMI-2\" geometry=1920x1080+0+0\n",
            )
        );
    }

    #[test]
    fn new_warning_report_suppresses_consecutive_repeated_messages() {
        let mut previous_warnings = HashSet::new();
        let mut stderr = Vec::new();
        let first_plan = EnforcementPlan {
            operations: Vec::new(),
            warnings: vec!["window not found: app_id:\"Player\"".to_string()],
        };
        let second_plan = EnforcementPlan {
            operations: Vec::new(),
            warnings: vec![
                "window not found: app_id:\"Player\"".to_string(),
                "window not found: app_id:\"Overlay\"".to_string(),
            ],
        };
        let clear_plan = EnforcementPlan::default();
        let reappeared_plan = EnforcementPlan {
            operations: Vec::new(),
            warnings: vec!["window not found: app_id:\"Player\"".to_string()],
        };

        write_new_warnings(&first_plan, &mut stderr, &mut previous_warnings)
            .expect("first warnings should render");
        write_new_warnings(&second_plan, &mut stderr, &mut previous_warnings)
            .expect("new warnings should render");
        write_new_warnings(&clear_plan, &mut stderr, &mut previous_warnings)
            .expect("cleared warnings should render");
        write_new_warnings(&reappeared_plan, &mut stderr, &mut previous_warnings)
            .expect("reappeared warnings should render");

        assert_eq!(
            String::from_utf8_lossy(&stderr),
            concat!(
                "warning: window not found: app_id:\"Player\"\n",
                "warning: window not found: app_id:\"Overlay\"\n",
                "warning: window not found: app_id:\"Player\"\n",
            )
        );
    }
}
