//! Project automation. Run with `cargo xtask <command>`.

mod codegen;
mod sha256;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("codegen") => codegen::run(args.iter().any(|a| a == "--check")),
        _ => {
            eprintln!("usage: cargo xtask codegen [--check]");
            ExitCode::FAILURE
        }
    }
}
