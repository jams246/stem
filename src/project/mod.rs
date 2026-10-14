//! Project discovery: the root folder and the candidate files tsconfig selects.

pub mod paths;
mod select;
mod tsconfig;
mod walk;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::StemError;

/// The project root and the files tsconfig makes candidates.
#[derive(Debug)]
pub struct Project {
    /// Absolute folder that holds the tsconfig, or the scanned folder.
    pub root: PathBuf,
    /// Candidate files relative to `root`, sorted by bytes.
    pub candidates: Vec<String>,
}

/// Finds the project root for `dir`: the nearest folder at or above it
/// holding a `tsconfig.json`. Returns the root and the tsconfig, if any.
pub fn find_root(dir: &Path) -> Result<(PathBuf, Option<PathBuf>), StemError> {
    let shown = paths::shown(dir);
    let absolute = std::path::absolute(dir).map_err(|e| StemError::read(shown.clone(), e))?;
    let absolute = paths::lexical(&absolute);
    let metadata = std::fs::metadata(&absolute).map_err(|e| StemError::read(shown.clone(), e))?;
    if !metadata.is_dir() {
        return Err(StemError::Usage(format!("{shown} is not a folder")));
    }
    let mut current = Some(absolute.as_path());
    while let Some(folder) = current {
        let candidate = folder.join("tsconfig.json");
        if candidate.is_file() {
            return Ok((folder.to_path_buf(), Some(candidate)));
        }
        current = folder.parent();
    }
    Ok((absolute, None))
}

/// Discovers the project for `dir` and lists its candidate files.
pub fn discover(dir: &Path) -> Result<Project, StemError> {
    let (root, tsconfig) = find_root(dir)?;
    let files = walk::list_files(&root);
    let Some(tsconfig) = tsconfig else {
        let candidates = files.into_iter().map(|f| f.rel).collect();
        return Ok(Project { root, candidates });
    };
    let mut selected = vec![false; files.len()];
    let mut queue = vec![tsconfig];
    let mut seen: HashSet<PathBuf> = HashSet::new();
    while let Some(config_path) = queue.pop() {
        if !seen.insert(config_path.clone()) {
            continue;
        }
        let project = tsconfig::load(&config_path)?;
        select::select(&project, &config_path, &files, &mut selected)?;
        queue.extend(project.references.iter().rev().cloned());
    }
    let candidates = files
        .into_iter()
        .zip(selected)
        .filter(|(_, keep)| *keep)
        .map(|(f, _)| f.rel)
        .collect();
    Ok(Project { root, candidates })
}
