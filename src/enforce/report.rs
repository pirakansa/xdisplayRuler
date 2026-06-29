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
