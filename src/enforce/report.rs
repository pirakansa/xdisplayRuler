use std::io::Write;

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

    use super::write_dry_run_report;

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
}
