mod app;
mod cli;
mod config;
mod csv;
mod error;
mod expr;
mod fs_overlay;
mod game;
mod instruction;
mod runtime;
mod script;
mod terminal;
mod vm;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::parse(std::env::args_os().skip(1)) {
        Ok(cli::Command::Help) => {
            println!("{}", cli::usage());
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Run(options)) => match app::run(options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("temuera-headless: {err}");
                ExitCode::from(err.exit_code())
            }
        },
        Err(err) => {
            eprintln!("temuera-headless: {err}");
            eprintln!("{}", cli::usage());
            ExitCode::from(err.exit_code())
        }
    }
}
