//! The `intent` command-line tool.

use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let code = intent_cli::run(
        std::env::args_os(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(code)
}
