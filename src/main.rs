use std::{env, io, process};

use xdisplay_ruler::cli::{self, CliExit};

fn main() {
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    match cli::run(env::args().skip(1), &mut stdout, &mut stderr) {
        Ok(CliExit::Success) => {}
        Ok(CliExit::UsageError) => process::exit(2),
        Err(error) => {
            eprintln!("xdisplay-ruler: {error}");
            process::exit(1);
        }
    }
}
