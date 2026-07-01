use std::{collections::HashSet, io::Write, thread, time::Duration};

use crate::{backend::WindowLayoutBackend, ConfiguredBackend};

use super::{executor::apply_plan, planner::EnforcementSession, report, EnforceOptions};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnforceCycleMode {
    DryRun,
    ApplyOnce,
}

pub(super) fn run(
    options: EnforceOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    build_backend: impl Fn(&str) -> Result<ConfiguredBackend, String>,
) -> Result<(), String> {
    run_with_layout_backend(options, stdout, stderr, build_backend)
}

pub(super) fn run_with_layout_backend<B>(
    options: EnforceOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    build_backend: impl Fn(&str) -> Result<B, String>,
) -> Result<(), String>
where
    B: WindowLayoutBackend,
{
    let single_cycle_mode = single_cycle_mode(&options);
    let loop_interval = Duration::from_millis(options.interval_millis as u64);
    let mut session = EnforcementSession::new(&options, build_backend)?;

    if let Some(mode) = single_cycle_mode {
        run_single_cycle(&mut session, mode, stdout, stderr)
    } else {
        run_daemon(&mut session, loop_interval, stderr)
    }
}

fn single_cycle_mode(options: &EnforceOptions) -> Option<EnforceCycleMode> {
    if options.dry_run {
        Some(EnforceCycleMode::DryRun)
    } else if options.once {
        Some(EnforceCycleMode::ApplyOnce)
    } else {
        None
    }
}

fn run_single_cycle(
    session: &mut EnforcementSession<impl WindowLayoutBackend>,
    mode: EnforceCycleMode,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let plan = session.build_recoverable_plan()?;
    report::write_warnings(&plan, stderr)?;

    match mode {
        EnforceCycleMode::DryRun => report::write_dry_run_report(&plan, stdout),
        EnforceCycleMode::ApplyOnce => apply_plan(session.backend(), &plan),
    }
}

fn run_daemon(
    session: &mut EnforcementSession<impl WindowLayoutBackend>,
    interval: Duration,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let mut previous_warnings = HashSet::new();
    loop {
        let plan = session.build_recoverable_plan()?;
        report::write_new_warnings(&plan, stderr, &mut previous_warnings)?;
        apply_plan(session.backend(), &plan)?;
        thread::sleep(interval);
    }
}
