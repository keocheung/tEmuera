use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{HeadlessError, Result};

const SPECIAL_DIRS: &[&str] = &["CSV", "ERB", "DAT", "DEBUG", "RESOURCES", "resources"];

#[derive(Debug)]
pub struct CaseOverlay {
    root: PathBuf,
}

impl CaseOverlay {
    pub fn prepare(source_root: &Path) -> Result<Self> {
        let source_root = source_root.canonicalize().map_err(|err| {
            HeadlessError::io(format!("canonicalize {}", source_root.display()), err)
        })?;
        let root = std::env::temp_dir()
            .join("temuera-headless")
            .join(stable_path_hash(&source_root));

        if root.exists() {
            fs::remove_dir_all(&root)
                .map_err(|err| HeadlessError::io(format!("remove {}", root.display()), err))?;
        }
        fs::create_dir_all(&root)
            .map_err(|err| HeadlessError::io(format!("create {}", root.display()), err))?;

        link_top_level_entries(&source_root, &root)?;
        materialize_special_dirs(&source_root, &root)?;

        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for CaseOverlay {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn link_top_level_entries(source_root: &Path, overlay_root: &Path) -> Result<()> {
    for entry in read_dir(source_root)? {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() && is_special_dir(&name) {
            continue;
        }

        let target = overlay_root.join(name.as_ref());
        symlink_path(&path, &target)?;
        add_alias_links(overlay_root, &name, &path)?;
    }

    Ok(())
}

fn materialize_special_dirs(source_root: &Path, overlay_root: &Path) -> Result<()> {
    let mut seen = HashSet::new();

    for requested_name in SPECIAL_DIRS {
        let Some(source_dir) = find_child_dir(source_root, requested_name)? else {
            continue;
        };
        let canonical = source_dir.canonicalize().map_err(|err| {
            HeadlessError::io(format!("canonicalize {}", source_dir.display()), err)
        })?;
        if !seen.insert(canonical) {
            continue;
        }

        let overlay_dir = overlay_root.join(requested_name);
        if overlay_dir.exists() {
            fs::remove_file(&overlay_dir)
                .or_else(|_| fs::remove_dir_all(&overlay_dir))
                .map_err(|err| {
                    HeadlessError::io(format!("replace {}", overlay_dir.display()), err)
                })?;
        }
        fs::create_dir_all(&overlay_dir)
            .map_err(|err| HeadlessError::io(format!("create {}", overlay_dir.display()), err))?;
        link_recursive_entries(&source_dir, &overlay_dir)?;
        add_alias_links(overlay_root, requested_name, &overlay_dir)?;
    }

    Ok(())
}

fn link_recursive_entries(source_dir: &Path, overlay_dir: &Path) -> Result<()> {
    for entry in read_dir(source_dir)? {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let target = overlay_dir.join(name.as_ref());

        if path.is_dir() {
            fs::create_dir_all(&target)
                .map_err(|err| HeadlessError::io(format!("create {}", target.display()), err))?;
            add_alias_links(overlay_dir, &name, &target)?;
            link_recursive_entries(&path, &target)?;
        } else {
            symlink_path(&path, &target)?;
            add_alias_links(overlay_dir, &name, &path)?;
        }
    }
    Ok(())
}

fn add_alias_links(parent: &Path, name: &str, source_path: &Path) -> Result<()> {
    for alias in [name.to_uppercase(), name.to_lowercase()] {
        let alias_path = parent.join(alias);
        if path_exists_or_symlink(&alias_path) {
            continue;
        }
        symlink_path(source_path, &alias_path)?;
    }
    Ok(())
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

fn is_special_dir(name: &str) -> bool {
    SPECIAL_DIRS
        .iter()
        .any(|special| name.eq_ignore_ascii_case(special))
}

fn read_dir(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .map_err(|err| HeadlessError::io(format!("read directory {}", path.display()), err))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| HeadlessError::io(format!("read directory {}", path.display()), err))?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn stable_path_hash(path: &Path) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn path_exists_or_symlink(path: &Path) -> bool {
    path.exists() || fs::symlink_metadata(path).is_ok()
}

#[cfg(unix)]
fn symlink_path(source: &Path, target: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, target).map_err(|err| {
        HeadlessError::io(
            format!("link {} -> {}", target.display(), source.display()),
            err,
        )
    })
}

#[cfg(windows)]
fn symlink_path(source: &Path, target: &Path) -> Result<()> {
    let result = if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, target)
    } else {
        std::os::windows::fs::symlink_file(source, target)
    };
    result.map_err(|err| {
        HeadlessError::io(
            format!("link {} -> {}", target.display(), source.display()),
            err,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_dirs_are_case_insensitive() {
        assert!(is_special_dir("erb"));
        assert!(is_special_dir("Resources"));
        assert!(!is_special_dir("sav"));
    }
}
