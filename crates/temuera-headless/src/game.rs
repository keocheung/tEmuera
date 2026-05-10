use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ConfigFile;
use crate::csv::CsvCatalog;
use crate::error::{HeadlessError, Result};
use crate::script::ScriptCatalog;

#[derive(Debug, Clone)]
pub struct GamePaths {
    pub source_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub csv_dir: Option<PathBuf>,
    pub erb_dir: Option<PathBuf>,
    pub dat_dir: Option<PathBuf>,
    pub debug_dir: Option<PathBuf>,
    pub resources_dir: Option<PathBuf>,
    pub sav_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceSummary {
    pub erb_files: usize,
    pub csv_files: usize,
    pub dat_files: usize,
    pub save_files: usize,
    pub has_config: bool,
    pub has_sav_dir: bool,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub paths: GamePaths,
    pub resources: ResourceSummary,
    pub config: ConfigFile,
    pub csv: CsvCatalog,
    pub scripts: ScriptCatalog,
}

impl Game {
    pub fn open(source_dir: PathBuf, runtime_dir: PathBuf) -> Result<Self> {
        if !source_dir.is_dir() {
            return Err(HeadlessError::InvalidGameDir(source_dir));
        }
        if !runtime_dir.is_dir() {
            return Err(HeadlessError::InvalidGameDir(runtime_dir));
        }

        let layout = GameLayout::discover(&runtime_dir)?;
        let resources = ResourceSummary::scan(&runtime_dir)?;
        let config = ConfigFile::load(&runtime_dir)?;
        let csv = layout
            .csv_dir
            .as_ref()
            .map(|path| CsvCatalog::load(path, &runtime_dir))
            .transpose()?
            .unwrap_or_default();
        let scripts = layout
            .erb_dir
            .as_ref()
            .map(|path| ScriptCatalog::load(path, &runtime_dir))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            paths: GamePaths {
                source_dir,
                runtime_dir,
                csv_dir: layout.csv_dir,
                erb_dir: layout.erb_dir,
                dat_dir: layout.dat_dir,
                debug_dir: layout.debug_dir,
                resources_dir: layout.resources_dir,
                sav_dir: layout.sav_dir,
            },
            resources,
            config,
            csv,
            scripts,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct GameLayout {
    csv_dir: Option<PathBuf>,
    erb_dir: Option<PathBuf>,
    dat_dir: Option<PathBuf>,
    debug_dir: Option<PathBuf>,
    resources_dir: Option<PathBuf>,
    sav_dir: Option<PathBuf>,
}

impl GameLayout {
    fn discover(root: &Path) -> Result<Self> {
        Ok(Self {
            csv_dir: find_child_dir(root, "csv")?,
            erb_dir: find_child_dir(root, "erb")?,
            dat_dir: find_child_dir(root, "dat")?,
            debug_dir: find_child_dir(root, "debug")?,
            resources_dir: find_child_dir(root, "resources")?,
            sav_dir: find_child_dir(root, "sav")?,
        })
    }
}

impl ResourceSummary {
    pub fn scan(root: &Path) -> Result<Self> {
        let mut summary = Self::default();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            for entry in read_dir(&dir)? {
                let path = entry.path();
                if path.is_dir() {
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("sav")
                    {
                        summary.has_sav_dir = true;
                    }
                    stack.push(path);
                    continue;
                }

                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                let extension = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();

                if file_name.eq_ignore_ascii_case("emuera.config") {
                    summary.has_config = true;
                }

                match extension.to_ascii_lowercase().as_str() {
                    "erb" => summary.erb_files += 1,
                    "csv" => summary.csv_files += 1,
                    "dat" => summary.dat_files += 1,
                    "sav" => summary.save_files += 1,
                    _ => {}
                }
            }
        }

        Ok(summary)
    }
}

fn read_dir(path: &Path) -> Result<Vec<fs::DirEntry>> {
    fs::read_dir(path)
        .map_err(|err| HeadlessError::io(format!("read directory {}", path.display()), err))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| HeadlessError::io(format!("read directory {}", path.display()), err))
}

fn find_child_dir(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    for entry in read_dir(root)? {
        let path = entry.path();
        if path.is_dir()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}
