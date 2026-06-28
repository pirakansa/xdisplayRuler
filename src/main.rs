use std::{env, process};

use display_ruler::DisplayState;

const HELP: &str = "\
display-ruler

Usage:
  display-ruler [--help] [--version]

The current build contains the display-state engine and prints the active
in-memory snapshot. Xorg/XRandR event collection is a planned backend.
";

fn main() {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        None => print!("{}", DisplayState::new().status_report()),
        Some("--help" | "-h") => print!("{HELP}"),
        Some("--version" | "-V") => println!("{}", env!("CARGO_PKG_VERSION")),
        Some(argument) => {
            eprintln!("unknown argument: {argument}");
            eprintln!("try --help");
            process::exit(2);
        }
    }
}
