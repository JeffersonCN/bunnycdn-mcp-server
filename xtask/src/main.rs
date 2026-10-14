//! Project automation. Run with `cargo xtask <command>`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("codegen") => {
            // Implemented in T01 (specs/tasks/T01-codegen.md).
            eprintln!("xtask codegen: not implemented yet (see T01)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: cargo xtask codegen [--check]");
            ExitCode::FAILURE
        }
    }
}
