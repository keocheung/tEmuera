use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::read_text_lossy;
use crate::error::{HeadlessError, Result};

#[derive(Debug, Clone, Default)]
pub struct CsvCatalog {
    pub files: Vec<CsvFile>,
    pub rows: usize,
    pub name_tables: HashMap<String, NameTable>,
}

#[derive(Debug, Clone)]
pub struct CsvFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub rows: Vec<CsvRow>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CsvRow {
    pub line_no: usize,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NameTable {
    pub source_file: PathBuf,
    pub by_id: HashMap<i64, String>,
    pub by_name: HashMap<String, i64>,
}

impl CsvCatalog {
    pub fn load(csv_dir: &Path, root: &Path) -> Result<Self> {
        if !csv_dir.is_dir() {
            return Ok(Self::default());
        }

        let mut files = Vec::new();
        for path in collect_files(csv_dir, "csv")? {
            let text = read_text_lossy(&path)?;
            let rows = parse_csv(&text);
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(path.as_path())
                .to_path_buf();
            files.push(CsvFile {
                path,
                relative_path,
                rows,
            });
        }
        files.sort_by_key(|file| file.relative_path.clone());
        let rows = files.iter().map(|file| file.rows.len()).sum();
        let name_tables = build_name_tables(&files);
        Ok(Self {
            files,
            rows,
            name_tables,
        })
    }

    pub fn get_file(&self, name: &str) -> Option<&CsvFile> {
        self.files.iter().find(|file| {
            file.path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|file_name| file_name.eq_ignore_ascii_case(name))
        })
    }

    pub fn resolve_name(&self, table: &str, name: &str) -> Option<i64> {
        self.name_tables
            .get(&table.to_ascii_uppercase())
            .and_then(|table| table.by_name.get(&name.to_ascii_uppercase()).copied())
    }

    pub fn name_for_id(&self, table: &str, id: i64) -> Option<&str> {
        self.name_tables
            .get(&table.to_ascii_uppercase())
            .and_then(|table| table.by_id.get(&id))
            .map(String::as_str)
    }
}

fn build_name_tables(files: &[CsvFile]) -> HashMap<String, NameTable> {
    let mut tables = HashMap::new();
    for file in files {
        let Some(stem) = file.path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let mut table = NameTable {
            source_file: file.relative_path.clone(),
            ..NameTable::default()
        };
        for row in &file.rows {
            if row.cells.len() < 2 {
                continue;
            }
            let Ok(id) = row.cells[0].trim().parse::<i64>() else {
                continue;
            };
            let name = row.cells[1].trim();
            if name.is_empty() || name.starts_with(';') {
                continue;
            }
            table.by_id.entry(id).or_insert_with(|| name.to_owned());
            table.by_name.entry(name.to_ascii_uppercase()).or_insert(id);
        }
        if !table.by_id.is_empty() {
            tables.insert(stem.to_ascii_uppercase(), table);
        }
    }
    tables
}

fn parse_csv(text: &str) -> Vec<CsvRow> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let cells = parse_csv_line(line);
            if cells.is_empty() {
                None
            } else {
                Some(CsvRow {
                    line_no: index + 1,
                    cells,
                })
            }
        })
        .collect()
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.trim_start_matches('\u{feff}').chars().peekable();
    let mut in_quotes = false;
    let mut only_whitespace = true;
    let mut stopped_by_comment = false;

    while let Some(ch) = chars.next() {
        if only_whitespace && !in_quotes && ch == ';' {
            stopped_by_comment = true;
            break;
        }
        match ch {
            '"' => {
                only_whitespace = false;
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                if current.trim_start().starts_with(';') {
                    stopped_by_comment = true;
                    break;
                }
                cells.push(current.trim().to_owned());
                current.clear();
                only_whitespace = true;
            }
            _ => {
                if !ch.is_whitespace() {
                    only_whitespace = false;
                }
                current.push(ch);
            }
        }
    }

    if current.trim_start().starts_with(';') {
        return cells;
    }
    let value = current.trim();
    if !stopped_by_comment && (!value.is_empty() || !cells.is_empty()) {
        cells.push(value.to_owned());
    }

    if cells.first().is_some_and(|cell| cell.starts_with(';')) {
        Vec::new()
    } else {
        cells
    }
}

pub fn collect_files(root: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let extension = extension.to_ascii_lowercase();

    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir)
            .map_err(|err| HeadlessError::io(format!("read directory {}", dir.display()), err))?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|err| HeadlessError::io(format!("read directory {}", dir.display()), err))?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(&extension))
            {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments_quotes_and_empty_cells() {
        assert_eq!(parse_csv_line(";comment"), Vec::<String>::new());
        assert_eq!(
            parse_csv_line("1,\"two, too\",,4,; comment"),
            vec!["1", "two, too", "", "4"]
        );
    }

    #[test]
    fn builds_name_tables_from_numeric_rows() {
        let file = CsvFile {
            path: PathBuf::from("Talent.csv"),
            relative_path: PathBuf::from("CSV/Talent.csv"),
            rows: vec![CsvRow {
                line_no: 1,
                cells: vec!["2".to_owned(), "性別".to_owned()],
            }],
        };
        let tables = build_name_tables(&[file]);
        let table = tables.get("TALENT").unwrap();
        assert_eq!(table.by_name.get("性別"), Some(&2));
        assert_eq!(table.by_id.get(&2).map(String::as_str), Some("性別"));
    }
}
