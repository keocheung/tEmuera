use std::collections::HashMap;

use crate::csv::CsvCatalog;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
}

impl Default for Value {
    fn default() -> Self {
        Self::Int(0)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct VarAddress {
    pub name: String,
    pub indexes: Vec<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct VarStore {
    values: HashMap<VarAddress, Value>,
}

impl VarStore {
    pub fn get(&self, address: &VarAddress) -> Value {
        self.values
            .get(address)
            .cloned()
            .unwrap_or_else(|| default_value_for(&address.name))
    }

    pub fn set(&mut self, address: VarAddress, value: Value) {
        self.values.insert(address, value);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

fn default_value_for(name: &str) -> Value {
    match name.to_ascii_uppercase().as_str() {
        "RESULTS" | "NAME" | "CALLNAME" | "NICKNAME" | "MASTERNAME" | "CSTR" | "STR" | "TSTR" => {
            Value::Str(String::new())
        }
        _ => Value::Int(0),
    }
}

#[derive(Debug, Clone)]
pub struct VarResolver<'a> {
    csv: &'a CsvCatalog,
}

impl<'a> VarResolver<'a> {
    pub fn new(csv: &'a CsvCatalog) -> Self {
        Self { csv }
    }

    pub fn csv(&self) -> &'a CsvCatalog {
        self.csv
    }

    pub fn parse_address(&self, text: &str) -> Option<VarAddress> {
        let mut parts = text.split(':');
        let name = normalize_identifier(parts.next()?.trim())?;
        let indexes = parts
            .map(str::trim)
            .map(|part| self.parse_index(&name, part))
            .collect::<Option<Vec<_>>>()?;
        Some(VarAddress { name, indexes })
    }

    fn parse_index(&self, variable_name: &str, part: &str) -> Option<i64> {
        if part.is_empty() {
            return None;
        }
        if let Ok(value) = part.parse::<i64>() {
            return Some(value);
        }
        if let Some(value) = self.csv.resolve_name(variable_name, part) {
            return Some(value);
        }

        // Character scoped variables commonly use ARG/TARGET/MASTER-like symbolic
        // indexes. Full expression evaluation will replace this fallback later.
        match part.to_ascii_uppercase().as_str() {
            "MASTER" | "TARGET" | "ARG" | "ASSI" => Some(0),
            _ => None,
        }
    }
}

fn normalize_identifier(text: &str) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some(text.to_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::csv::{CsvCatalog, CsvFile, CsvRow};

    use super::*;

    fn catalog_with_talent() -> CsvCatalog {
        let file = CsvFile {
            path: PathBuf::from("Talent.csv"),
            relative_path: PathBuf::from("CSV/Talent.csv"),
            rows: vec![CsvRow {
                line_no: 1,
                cells: vec!["2".to_owned(), "性別".to_owned()],
            }],
        };
        let mut catalog = CsvCatalog {
            files: vec![file],
            rows: 1,
            name_tables: HashMap::new(),
        };
        catalog.name_tables.insert(
            "TALENT".to_owned(),
            crate::csv::NameTable {
                source_file: PathBuf::from("CSV/Talent.csv"),
                by_id: HashMap::from([(2, "性別".to_owned())]),
                by_name: HashMap::from([("性別".to_owned(), 2)]),
            },
        );
        catalog
    }

    #[test]
    fn resolves_csv_named_indexes() {
        let catalog = catalog_with_talent();
        let resolver = VarResolver::new(&catalog);
        let address = resolver.parse_address("TALENT:ARG:性別").unwrap();
        assert_eq!(
            address,
            VarAddress {
                name: "TALENT".to_owned(),
                indexes: vec![0, 2],
            }
        );
    }

    #[test]
    fn stores_values_by_address() {
        let address = VarAddress {
            name: "FLAG".to_owned(),
            indexes: vec![1],
        };
        let mut store = VarStore::default();
        assert_eq!(store.get(&address), Value::Int(0));
        store.set(address.clone(), Value::Int(42));
        assert_eq!(store.get(&address), Value::Int(42));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn defaults_string_variables_to_empty_string() {
        let store = VarStore::default();
        let address = VarAddress {
            name: "NAME".to_owned(),
            indexes: vec![0],
        };
        assert_eq!(store.get(&address), Value::Str(String::new()));
    }
}
