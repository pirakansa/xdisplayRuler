use std::io::{self, Write};

use crate::DisplayState;

const HELP: &str = "\
display-ruler

Usage:
  display-ruler [--help] [--version]

The current build contains the display-state engine and prints the active
in-memory snapshot. Xorg/XRandR event collection is a planned backend.
";

#[derive(Debug, Eq, PartialEq)]
pub enum CliExit {
    Success,
    UsageError,
}

pub fn run<I, S>(
    arguments: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<CliExit>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut arguments = arguments.into_iter();

    match arguments.next().as_ref().map(AsRef::as_ref) {
        None => {
            write!(stdout, "{}", DisplayState::new().status_report())?;
            Ok(CliExit::Success)
        }
        Some("--help" | "-h") => {
            write!(stdout, "{HELP}")?;
            Ok(CliExit::Success)
        }
        Some("--version" | "-V") => {
            writeln!(stdout, "{}", env!("CARGO_PKG_VERSION"))?;
            Ok(CliExit::Success)
        }
        Some(argument) => {
            writeln!(stderr, "unknown argument: {argument}")?;
            writeln!(stderr, "try --help")?;
            Ok(CliExit::UsageError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{run, CliExit};

    #[test]
    fn reports_usage_errors_for_unknown_arguments() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let exit = run(["--bad-option"], &mut stdout, &mut stderr).expect("cli should run");

        assert_eq!(exit, CliExit::UsageError);
        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "unknown argument: --bad-option\ntry --help\n"
        );
    }
}
