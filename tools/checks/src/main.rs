mod cli;
mod commands;
mod digits;
mod enforce;
mod family;
mod pytext;
mod repo;

use std::process::ExitCode;

fn main() -> ExitCode {
    let raw: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let mut args = Vec::with_capacity(raw.len());
    for arg in raw {
        match arg.into_string() {
            Ok(text) => args.push(text),
            Err(bytes) => {
                eprintln!("checks: argument is not valid UTF-8: {bytes:?}");
                return ExitCode::from(2);
            }
        }
    }
    let mut args = args.into_iter();
    let Some(name) = args.next() else {
        eprintln!("usage: checks <command> [args]\n{}", commands::listing());
        return ExitCode::from(2);
    };
    if matches!(name.as_str(), "--help" | "-h" | "help") {
        println!("usage: checks <command> [args]\n{}", commands::listing());
        return ExitCode::SUCCESS;
    }
    let rest: Vec<String> = args.collect();
    match commands::find(&name) {
        Some(command) => ExitCode::from((command.run)(&rest)),
        None => {
            eprintln!("unknown command: {name}\n{}", commands::listing());
            ExitCode::from(2)
        }
    }
}
