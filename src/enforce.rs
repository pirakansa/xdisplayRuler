use std::{io::Write, thread, time::Duration};

use crate::ConfiguredBackend;

mod executor;
mod planner;
mod report;

use executor::apply_plan;
use planner::EnforcementSession;
use report::{write_dry_run_report, write_warnings};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EnforceOptions {
    pub(crate) backend_name: String,
    pub(crate) layout_path: String,
    pub(crate) once: bool,
    pub(crate) dry_run: bool,
    pub(crate) interval_millis: usize,
}

impl EnforceOptions {
    fn single_cycle_mode(&self) -> Option<EnforceCycleMode> {
        if self.dry_run {
            Some(EnforceCycleMode::DryRun)
        } else if self.once {
            Some(EnforceCycleMode::ApplyOnce)
        } else {
            None
        }
    }

    fn loop_interval(&self) -> Duration {
        Duration::from_millis(self.interval_millis as u64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnforceCycleMode {
    DryRun,
    ApplyOnce,
}

pub(crate) fn run(
    options: EnforceOptions,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    build_backend: impl Fn(&str) -> Result<ConfiguredBackend, String>,
) -> Result<(), String> {
    let single_cycle_mode = options.single_cycle_mode();
    let loop_interval = options.loop_interval();
    let mut session = EnforcementSession::new(&options, build_backend)?;

    if let Some(mode) = single_cycle_mode {
        run_single_cycle(&mut session, mode, stdout, stderr)
    } else {
        run_daemon(&mut session, loop_interval, stderr)
    }
}

fn run_single_cycle(
    session: &mut EnforcementSession,
    mode: EnforceCycleMode,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), String> {
    let plan = match mode {
        EnforceCycleMode::DryRun => session.build_recoverable_plan()?,
        EnforceCycleMode::ApplyOnce => session.build_strict_plan()?,
    };
    write_warnings(&plan, stderr)?;

    match mode {
        EnforceCycleMode::DryRun => write_dry_run_report(&plan, stdout),
        EnforceCycleMode::ApplyOnce => apply_plan(session.backend(), &plan),
    }
}

fn run_daemon(
    session: &mut EnforcementSession,
    interval: Duration,
    stderr: &mut impl Write,
) -> Result<(), String> {
    loop {
        let plan = session.build_recoverable_plan()?;
        write_warnings(&plan, stderr)?;
        apply_plan(session.backend(), &plan)?;
        thread::sleep(interval);
    }
}
