use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::{HeadlessError, Result};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    Help,
    Run(Options),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Options {
    pub game_dir: PathBuf,
    pub show_warnings: bool,
    pub no_overlay: bool,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command> {
    let mut game_dir = None;
    let mut show_warnings = false;
    let mut no_overlay = false;

    for arg in args {
        match arg.to_str() {
            Some("-h" | "--help") => return Ok(Command::Help),
            Some("--show-warnings") => show_warnings = true,
            Some("--no-overlay") => no_overlay = true,
            Some(flag) if flag.starts_with('-') => {
                return Err(HeadlessError::Usage(format!("unknown option: {flag}")));
            }
            _ => {
                if game_dir.replace(PathBuf::from(arg)).is_some() {
                    return Err(HeadlessError::Usage(
                        "only one game directory can be supplied".to_owned(),
                    ));
                }
            }
        }
    }

    let Some(game_dir) = game_dir else {
        return Err(HeadlessError::Usage(
            "missing required game directory".to_owned(),
        ));
    };

    Ok(Command::Run(Options {
        game_dir,
        show_warnings,
        no_overlay,
    }))
}

pub fn usage() -> &'static str {
    "Usage: cargo run -p temuera-headless -- [--show-warnings] [--no-overlay] /path/to/era-game\n\
\n\
Options:\n\
  --show-warnings   keep loader/parser warnings visible\n\
  --no-overlay      use the game directory directly instead of a case-insensitive temp overlay\n\
  -h, --help        show this help"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_flags_and_game_dir() {
        let command = parse(os(&["--show-warnings", "--no-overlay", "game"])).unwrap();
        assert_eq!(
            command,
            Command::Run(Options {
                game_dir: PathBuf::from("game"),
                show_warnings: true,
                no_overlay: true,
            })
        );
    }

    #[test]
    fn rejects_multiple_game_dirs() {
        assert!(parse(os(&["game-a", "game-b"])).is_err());
    }
}
