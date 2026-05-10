use std::io::{self, Write};

use crate::cli::Options;
use crate::error::{HeadlessError, Result};
use crate::expr;
use crate::fs_overlay::CaseOverlay;
use crate::game::Game;
use crate::runtime::{Value, VarResolver, VarStore};
use crate::terminal::{Rgb, Terminal, TextStyle, emuera_columns};
use crate::vm::{StopReason, Vm};

pub fn run(options: Options) -> Result<()> {
    let source_dir = options.game_dir.canonicalize().map_err(|err| {
        HeadlessError::io(format!("canonicalize {}", options.game_dir.display()), err)
    })?;

    let overlay = if options.no_overlay {
        None
    } else {
        Some(CaseOverlay::prepare(&source_dir)?)
    };
    let runtime_dir = overlay
        .as_ref()
        .map(|overlay| overlay.root().to_path_buf())
        .unwrap_or_else(|| source_dir.clone());

    let game = Game::open(source_dir, runtime_dir)?;
    let mut terminal = Terminal::stdout();
    terminal.apply_palette(Rgb::EMUERA_ORANGE, Rgb::BLACK)?;
    terminal.clear()?;
    print_banner(&mut terminal, &game, options.show_warnings)?;
    terminal.flush()?;

    command_loop(&game)?;
    terminal.reset()?;
    Ok(())
}

fn print_banner<W: Write>(
    terminal: &mut Terminal<W>,
    game: &Game,
    show_warnings: bool,
) -> Result<()> {
    let normal = TextStyle {
        foreground: Some(Rgb::EMUERA_ORANGE),
        background: Some(Rgb::BLACK),
        ..TextStyle::default()
    };
    let heading = TextStyle {
        bold: true,
        ..normal
    };

    let title = "tEmuera Rust headless";
    terminal.writeln_styled(title, heading)?;
    terminal.writeln_styled(&"-".repeat(emuera_columns(title).max(40)), normal)?;
    terminal.writeln_styled(
        &format!("source : {}", game.paths.source_dir.display()),
        normal,
    )?;
    terminal.writeln_styled(
        &format!("runtime: {}", game.paths.runtime_dir.display()),
        normal,
    )?;
    terminal.writeln_styled(
        &format!(
            "resources: {} ERB, {} CSV, {} DAT, {} SAV",
            game.resources.erb_files,
            game.resources.csv_files,
            game.resources.dat_files,
            game.resources.save_files
        ),
        normal,
    )?;
    terminal.writeln_styled(
        &format!(
            "config: {}, sav dir: {}, warnings: {}",
            yes_no(game.resources.has_config),
            yes_no(game.resources.has_sav_dir),
            if show_warnings { "visible" } else { "hidden" }
        ),
        normal,
    )?;
    if let Some(path) = &game.config.path {
        terminal.writeln_styled(&format!("config path: {}", path.display()), normal)?;
    }
    if let Some(foreground) = game
        .config
        .get("文字色")
        .or_else(|| game.config.get("TextColor"))
    {
        terminal.writeln_styled(&format!("config text color: {foreground}"), normal)?;
    }
    terminal.writeln_styled(
        &format!(
            "loaded: {} config entries, {} CSV rows, {} script files, {} functions, {} labels",
            game.config.entries.len(),
            game.csv.rows,
            game.scripts.files.len(),
            game.scripts.index.functions.len(),
            game.scripts.index.labels.len()
        ),
        normal,
    )?;
    if let Some(title) = game
        .csv
        .get_file("GameBase.csv")
        .and_then(|file| {
            file.rows
                .iter()
                .find(|row| row.cells.first().is_some_and(|cell| cell == "タイトル"))
        })
        .and_then(|row| row.cells.get(1))
    {
        terminal.writeln_styled(&format!("title: {title}"), normal)?;
    }
    terminal.writeln_styled("", normal)?;
    terminal.writeln_styled(
        "Rust VM is not implemented yet. Type :help for available bootstrap commands.",
        normal,
    )
}

fn command_loop(game: &Game) -> Result<()> {
    let stdin = io::stdin();
    let mut line = String::new();
    let resolver = VarResolver::new(&game.csv);
    let mut variables = VarStore::default();

    loop {
        print!("[input] ");
        io::stdout()
            .flush()
            .map_err(|err| HeadlessError::io("flush prompt", err))?;

        line.clear();
        let bytes = stdin
            .read_line(&mut line)
            .map_err(|err| HeadlessError::io("read input", err))?;
        if bytes == 0 {
            break;
        }

        match line.trim_end() {
            ":q" | ":quit" => break,
            ":help" => print_help(),
            ":clear" => {
                let mut terminal = Terminal::stdout();
                terminal.apply_palette(Rgb::EMUERA_ORANGE, Rgb::BLACK)?;
                terminal.clear()?;
                terminal.flush()?;
            }
            ":paths" => {
                println!("source : {}", game.paths.source_dir.display());
                println!("runtime: {}", game.paths.runtime_dir.display());
                print_optional_path("csv", &game.paths.csv_dir);
                print_optional_path("erb", &game.paths.erb_dir);
                print_optional_path("dat", &game.paths.dat_dir);
                print_optional_path("debug", &game.paths.debug_dir);
                print_optional_path("resources", &game.paths.resources_dir);
                print_optional_path("sav", &game.paths.sav_dir);
            }
            ":scan" => {
                let resources = &game.resources;
                println!(
                    "resources: {} ERB, {} CSV, {} DAT, {} SAV, config={}, sav_dir={}",
                    resources.erb_files,
                    resources.csv_files,
                    resources.dat_files,
                    resources.save_files,
                    yes_no(resources.has_config),
                    yes_no(resources.has_sav_dir)
                );
                println!(
                    "loaded: {} config entries, {} CSV files, {} CSV rows, {} name tables, {} script files, {} enabled script lines",
                    game.config.entries.len(),
                    game.csv.files.len(),
                    game.csv.rows,
                    game.csv.name_tables.len(),
                    game.scripts.files.len(),
                    game.scripts.enabled_lines
                );
                let erb_files = game
                    .scripts
                    .files
                    .iter()
                    .filter(|file| matches!(file.kind, crate::script::ScriptFileKind::Erb))
                    .count();
                let erh_files = game.scripts.files.len().saturating_sub(erb_files);
                let script_lines: usize =
                    game.scripts.files.iter().map(|file| file.lines.len()).sum();
                println!(
                    "scripts: {erb_files} ERB, {erh_files} ERH, {script_lines} physical lines"
                );
                if let Some(first) = game.scripts.files.first() {
                    println!(
                        "first script: {} ({})",
                        first.relative_path.display(),
                        first.path.display()
                    );
                }
            }
            ":functions" => {
                for function in game.scripts.index.functions.iter().take(40) {
                    println!(
                        "{} @ {}:{}",
                        function.name,
                        function.location.file.display(),
                        function.location.line_no
                    );
                }
                if game.scripts.index.functions.len() > 40 {
                    println!("... {} more", game.scripts.index.functions.len() - 40);
                }
            }
            command if command.starts_with(":find ") => {
                let name = command.trim_start_matches(":find ").trim();
                if let Some(function) = game.scripts.find_function(name) {
                    println!(
                        "{} @ {}:{}",
                        function.name,
                        function.location.file.display(),
                        function.location.line_no
                    );
                } else {
                    println!("function not found: {name}");
                }
            }
            command if command.starts_with(":parse ") => {
                let name = command.trim_start_matches(":parse ").trim();
                if let Some(lines) = game.scripts.function_lines(name) {
                    for located in lines.into_iter().take(40) {
                        let instruction = crate::instruction::parse_line(&located.line.text);
                        println!(
                            "{}:{} {:?}",
                            located.file.display(),
                            located.line.line_no,
                            instruction
                        );
                    }
                } else {
                    println!("function not found: {name}");
                }
            }
            command if command.starts_with(":run ") => {
                let mut parts = command.trim_start_matches(":run ").split_whitespace();
                let name = parts.next().unwrap_or_default();
                let limit = parts
                    .next()
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(200);
                if name.is_empty() {
                    println!("usage: :run FUNCTION [STEP_LIMIT]");
                    continue;
                }
                let mut vm = Vm::new(&game.scripts, &game.csv);
                let report = vm.run_function(name, limit);
                for line in &report.output {
                    println!("{line}");
                }
                println!(
                    "vm stopped after {} steps: {}",
                    report.steps,
                    stop_reason_text(&report.stop_reason)
                );
            }
            command if command.starts_with(":run-with ") => {
                let mut parts = command.trim_start_matches(":run-with ").split_whitespace();
                let name = parts.next().unwrap_or_default();
                if name.is_empty() {
                    println!("usage: :run-with FUNCTION INPUT...");
                    continue;
                }
                let mut vm = Vm::new(&game.scripts, &game.csv);
                for input in parts {
                    vm.push_input(input);
                }
                let report = vm.run_function(name, 2000);
                for line in &report.output {
                    println!("{line}");
                }
                println!(
                    "vm stopped after {} steps: {}",
                    report.steps,
                    stop_reason_text(&report.stop_reason)
                );
            }
            command if command.starts_with(":name ") => {
                let mut parts = command.trim_start_matches(":name ").splitn(2, ' ');
                let table = parts.next().unwrap_or_default();
                let name = parts.next().unwrap_or_default();
                if table.is_empty() || name.is_empty() {
                    println!("usage: :name TABLE NAME");
                } else if let Some(id) = game.csv.resolve_name(table, name) {
                    let canonical = game.csv.name_for_id(table, id).unwrap_or(name);
                    let source = game
                        .csv
                        .name_tables
                        .get(&table.to_ascii_uppercase())
                        .map(|table| table.source_file.display().to_string())
                        .unwrap_or_else(|| "<unknown>".to_owned());
                    println!("{table}:{canonical} = {id} ({source})");
                } else {
                    println!("name not found: {table}:{name}");
                }
            }
            command if command.starts_with(":var ") => {
                let expression = command.trim_start_matches(":var ").trim();
                if let Some((address_text, value_text)) = expression.split_once('=') {
                    let Some(address) = resolver.parse_address(address_text.trim()) else {
                        println!("invalid variable address: {}", address_text.trim());
                        continue;
                    };
                    let value_text = value_text.trim();
                    let value = value_text
                        .parse::<i64>()
                        .map(Value::Int)
                        .unwrap_or_else(|_| Value::Str(value_text.to_owned()));
                    variables.set(address.clone(), value.clone());
                    println!("{address:?} = {value:?} ({} stored)", variables.len());
                } else if let Some(address) = resolver.parse_address(expression) {
                    println!("{address:?} = {:?}", variables.get(&address));
                } else {
                    println!("usage: :var ADDRESS[=VALUE]");
                }
            }
            command if command.starts_with(":eval ") => {
                let expression = command.trim_start_matches(":eval ").trim();
                match expr::parse(expression)
                    .and_then(|expr| expr::eval(&expr, &resolver, &variables))
                {
                    Ok(value) => println!("{value:?}"),
                    Err(err) => println!("eval error: {err:?}"),
                }
            }
            "" => {}
            input => {
                println!(
                    "Rust script execution is not wired yet; received input {:?}.",
                    input
                );
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("[headless]");
    println!("  :scan   rescan summary captured at startup");
    println!("  :paths  show source and runtime directories");
    println!("  :functions  list the first loaded ERB functions");
    println!("  :find NAME  find a loaded ERB function");
    println!("  :parse NAME  parse the first lines of a loaded ERB function");
    println!("  :run NAME [STEPS]  run a function with the bootstrap VM");
    println!("  :run-with NAME INPUT...  run with queued INPUT values");
    println!("  :name TABLE NAME  resolve a CSV name, e.g. :name TALENT 性別");
    println!("  :var ADDRESS[=VALUE]  inspect or set a runtime variable");
    println!("  :eval EXPR  evaluate a bootstrap expression");
    println!("  :clear  clear the terminal");
    println!("  :quit   exit");
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn print_optional_path(label: &str, path: &Option<std::path::PathBuf>) {
    match path {
        Some(path) => println!("{label}: {}", path.display()),
        None => println!("{label}: <missing>"),
    }
}

fn stop_reason_text(reason: &StopReason) -> String {
    match reason {
        StopReason::EndOfFunction => "end of function".to_owned(),
        StopReason::NeedInput => "need input".to_owned(),
        StopReason::FunctionNotFound(name) => format!("function not found: {name}"),
        StopReason::StepLimit => "step limit".to_owned(),
        StopReason::Error(message) => format!("error: {message}"),
        StopReason::Unsupported {
            file,
            line_no,
            text,
        } => format!("unsupported at {}:{}: {}", file.display(), line_no, text),
    }
}
