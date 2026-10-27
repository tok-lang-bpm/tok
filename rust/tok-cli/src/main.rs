//! The `tok` program; the commands are implemented in the library of this crate.

use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(tok_cli::main_with_args(&argv))
}
