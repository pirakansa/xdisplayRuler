use std::{io::Write, thread, time::Duration};

use crate::{
    build_enforcement_plan, ConfiguredBackend, DisplayState, EnforcementMode, EnforcementPlan,
    LayoutOperation, LayoutPolicy,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EnforceOptions {
    pub(crate) backend_name: String,
    pub(crate) layout_path: String,
    pub(crate) once: bool,
    pub(crate) dry_run: bool,
    pub(crate) interval_millis: usize,
}

pub(crate) fn run(
    options: EnforceOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    build_backend: impl Fn(&str) -> Result<ConfiguredBackend, String>,
) -> Result<(), String> {
    let policy =
        LayoutPolicy::read_from_path(&options.layout_path).map_err(|error| error.to_string())?;
    let mut backend = build_backend(&options.backend_name)?;
    let mut state = DisplayState::new();

    if options.dry_run || options.once {
        let mode = if options.once {
            EnforcementMode::Once
        } else {
            EnforcementMode::Daemon
        };
        let plan = plan_cycle(&mut backend, &mut state, &policy, mode)?;
        write_warnings(&plan, stderr)?;

        if options.dry_run {
            write!(stdout, "{}", plan_report(&plan)).map_err(|error| error.to_string())?;
        } else {
            apply_plan(&backend, &plan)?;
        }

        return Ok(());
    }

    loop {
        let plan = plan_cycle(&mut backend, &mut state, &policy, EnforcementMode::Daemon)?;
        write_warnings(&plan, stderr)?;
        apply_plan(&backend, &plan)?;
        thread::sleep(Duration::from_millis(options.interval_millis as u64));
    }
}

fn plan_cycle(
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

fn apply_plan(backend: &ConfiguredBackend, plan: &EnforcementPlan) -> Result<(), String> {
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
