use std::io::Write;

use crate::ConfiguredBackend;

mod executor;
mod planner;
mod report;
mod runner;

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
    runner::run(options, stdout, stderr, build_backend)
}

#[cfg(test)]
mod tests;
