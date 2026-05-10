use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::read_text_lossy;
use crate::csv::collect_files;
use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct ScriptCatalog {
    pub files: Vec<ScriptFile>,
    pub index: ScriptIndex,
    pub enabled_lines: usize,
}

#[derive(Debug, Clone)]
pub struct ScriptFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub kind: ScriptFileKind,
    pub lines: Vec<ScriptLine>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ScriptFileKind {
    Erb,
    Erh,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScriptLine {
    pub line_no: usize,
    pub text: String,
    pub kind: ScriptLineKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ScriptLineKind {
    Empty,
    Comment,
    Function,
    Label,
    Directive,
    Instruction,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptIndex {
    pub functions: Vec<FunctionDef>,
    pub labels: HashMap<String, ScriptLocation>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub location: ScriptLocation,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScriptLocation {
    pub file: PathBuf,
    pub line_no: usize,
}

impl ScriptCatalog {
    pub fn load(erb_dir: &Path, root: &Path) -> Result<Self> {
        if !erb_dir.is_dir() {
            return Ok(Self::default());
        }

        let mut paths = collect_files(erb_dir, "erb")?;
        paths.extend(collect_files(erb_dir, "erh")?);
        paths.sort();

        let mut files = Vec::new();
        let mut index = ScriptIndex::default();
        let mut enabled_lines = 0;

        for path in paths {
            let text = read_text_lossy(&path)?;
            let kind = ScriptFileKind::from_path(&path);
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(path.as_path())
                .to_path_buf();
            let lines = parse_script_lines(&text);

            for line in &lines {
                if !matches!(line.kind, ScriptLineKind::Empty | ScriptLineKind::Comment) {
                    enabled_lines += 1;
                }
                match line.kind {
                    ScriptLineKind::Function => {
                        if let Some(name) = parse_function_name(&line.text) {
                            index.functions.push(FunctionDef {
                                name,
                                location: ScriptLocation {
                                    file: relative_path.clone(),
                                    line_no: line.line_no,
                                },
                            });
                        }
                    }
                    ScriptLineKind::Label => {
                        if let Some(name) = parse_label_name(&line.text) {
                            index.labels.entry(name).or_insert_with(|| ScriptLocation {
                                file: relative_path.clone(),
                                line_no: line.line_no,
                            });
                        }
                    }
                    _ => {}
                }
            }

            files.push(ScriptFile {
                path,
                relative_path,
                kind,
                lines,
            });
        }

        index.functions.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.location.file.cmp(&b.location.file))
        });

        Ok(Self {
            files,
            index,
            enabled_lines,
        })
    }

    pub fn find_function(&self, name: &str) -> Option<&FunctionDef> {
        self.index
            .functions
            .iter()
            .find(|function| function.name.eq_ignore_ascii_case(name))
    }

    pub fn function_lines(&self, name: &str) -> Option<Vec<LocatedScriptLine<'_>>> {
        let function = self.find_function(name)?;
        let file = self
            .files
            .iter()
            .find(|file| file.relative_path == function.location.file)?;
        let start = file
            .lines
            .iter()
            .position(|line| line.line_no == function.location.line_no)?;

        let mut lines = Vec::new();
        for line in file.lines.iter().skip(start + 1) {
            if line.kind == ScriptLineKind::Function {
                break;
            }
            lines.push(LocatedScriptLine {
                file: &file.relative_path,
                line,
            });
        }
        Some(lines)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocatedScriptLine<'a> {
    pub file: &'a Path,
    pub line: &'a ScriptLine,
}

impl ScriptFileKind {
    fn from_path(path: &Path) -> Self {
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("erh"))
        {
            Self::Erh
        } else {
            Self::Erb
        }
    }
}

fn parse_script_lines(text: &str) -> Vec<ScriptLine> {
    text.lines()
        .enumerate()
        .map(|(index, text)| ScriptLine {
            line_no: index + 1,
            text: text.to_owned(),
            kind: classify_line(text),
        })
        .collect()
}

fn classify_line(line: &str) -> ScriptLineKind {
    let trimmed = line.trim_start().trim_start_matches('\u{feff}');
    if trimmed.is_empty() {
        ScriptLineKind::Empty
    } else if trimmed.starts_with(';') {
        ScriptLineKind::Comment
    } else if trimmed.starts_with('@') {
        ScriptLineKind::Function
    } else if trimmed.starts_with('$') {
        ScriptLineKind::Label
    } else if trimmed.starts_with('#') || (trimmed.starts_with('[') && trimmed.ends_with(']')) {
        ScriptLineKind::Directive
    } else {
        ScriptLineKind::Instruction
    }
}

fn parse_function_name(line: &str) -> Option<String> {
    parse_symbol_after_marker(line, '@')
}

fn parse_label_name(line: &str) -> Option<String> {
    parse_symbol_after_marker(line, '$')
}

fn parse_symbol_after_marker(line: &str, marker: char) -> Option<String> {
    let trimmed = line.trim_start().trim_start_matches('\u{feff}');
    let rest = trimmed.strip_prefix(marker)?.trim_start();
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '(' || ch == ';')
        .unwrap_or(rest.len());
    let name = rest[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_erb_lines() {
        assert_eq!(classify_line(";x"), ScriptLineKind::Comment);
        assert_eq!(classify_line("@EVENTFIRST"), ScriptLineKind::Function);
        assert_eq!(classify_line("$LOOP"), ScriptLineKind::Label);
        assert_eq!(classify_line("#DIM X"), ScriptLineKind::Directive);
        assert_eq!(classify_line("PRINTL hi"), ScriptLineKind::Instruction);
    }

    #[test]
    fn parses_function_names() {
        assert_eq!(
            parse_function_name("@FOO(ARG, BAR) ; comment"),
            Some("FOO".to_owned())
        );
    }
}
