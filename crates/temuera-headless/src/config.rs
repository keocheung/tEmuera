use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{HeadlessError, Result};

#[derive(Debug, Clone, Default)]
pub struct ConfigFile {
    pub path: Option<PathBuf>,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConfigEntry {
    pub line_no: usize,
    pub key: String,
    pub value: String,
}

impl ConfigFile {
    pub fn load(root: &Path) -> Result<Self> {
        let Some(path) = find_config(root)? else {
            return Ok(Self::default());
        };
        let text = read_text_lossy(&path)?;
        Ok(Self {
            path: Some(path),
            entries: parse_config(&text),
        })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
            .map(|entry| entry.value.as_str())
    }
}

fn find_config(root: &Path) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir(root)
        .map_err(|err| HeadlessError::io(format!("read directory {}", root.display()), err))?
    {
        let entry = entry
            .map_err(|err| HeadlessError::io(format!("read directory {}", root.display()), err))?;
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("emuera.config")
        {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

fn parse_config(text: &str) -> Vec<ConfigEntry> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| parse_config_line(index + 1, line))
        .collect()
}

fn parse_config_line(line_no: usize, line: &str) -> Option<ConfigEntry> {
    let line = line.trim().trim_start_matches('\u{feff}');
    if line.is_empty() || line.starts_with(';') {
        return None;
    }

    let separator = line.find(':').or_else(|| line.find('='))?;
    let key = line[..separator].trim();
    let value = line[separator + 1..].trim();
    if key.is_empty() {
        return None;
    }

    Some(ConfigEntry {
        line_no,
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

pub fn read_text_lossy(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).map_err(|err| HeadlessError::io(format!("read {}", path.display()), err))?;
    Ok(String::from_utf8_lossy(&bytes)
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colon_and_equals_entries() {
        let entries = parse_config("文字色:255,128,64\nFoo = Bar\n; ignored\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "文字色");
        assert_eq!(entries[0].value, "255,128,64");
        assert_eq!(entries[1].key, "Foo");
        assert_eq!(entries[1].value, "Bar");
    }
}
