use std::io::{self, Write};

mod command;
mod help;
mod options;
mod report;

pub use options::CliExit;

use command::{
    handle_command_result, handle_mode_command_result, run_configure_command, run_enforce_command,
    run_mode_command, run_modes_command, run_place_command, run_snapshot, run_stack_command,
    run_watch, StackCommand,
};
use help::HELP;
use options::{parse_options, Command};

pub fn run<I, S>(
    arguments: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<CliExit>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments = arguments
        .into_iter()
        .map(|argument| argument.as_ref().to_string())
        .collect::<Vec<_>>();

    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        write!(stdout, "{HELP}")?;
        return Ok(CliExit::Success);
    }

    if arguments
        .iter()
        .any(|argument| argument == "--version" || argument == "-V")
    {
        writeln!(stdout, "{}", env!("CARGO_PKG_VERSION"))?;
        return Ok(CliExit::Success);
    }

    let options = match parse_options(&arguments) {
        Ok(options) => options,
        Err(message) => {
            writeln!(stderr, "{message}")?;
            writeln!(stderr, "try --help")?;
            return Ok(CliExit::UsageError);
        }
    };

    match options.command {
        Command::Snapshot => {
            handle_command_result(run_snapshot(&options.backend_name, stdout), stderr)
        }
        Command::Watch => handle_command_result(run_watch(options, stdout), stderr),
        Command::Modes => handle_command_result(run_modes_command(options, stdout), stderr),
        Command::Mode => handle_mode_command_result(run_mode_command(options), stderr),
        Command::Enforce => {
            handle_command_result(run_enforce_command(options, stdout, stderr), stderr)
        }
        Command::Place => handle_command_result(run_place_command(options), stderr),
        Command::Configure => handle_command_result(run_configure_command(options), stderr),
        Command::Raise => {
            handle_command_result(run_stack_command(options, StackCommand::Raise), stderr)
        }
        Command::Lower => {
            handle_command_result(run_stack_command(options, StackCommand::Lower), stderr)
        }
    }
}

#[cfg(test)]
mod tests;
