//! Listing TypeScript files under the project root.

use std::path::Path;

use walkdir::WalkDir;

use super::paths::{is_typescript, relative, slash};

/// A TypeScript file found under the project root.
#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Path relative to the project root, `/`-separated.
    pub rel: String,
    /// Absolute path, `/`-separated, used to match tsconfig patterns.
    pub abs: String,
}

/// Lists every TypeScript source file under `root`.
///
/// Symlinks are never followed and `node_modules` folders are not entered,
/// since the built-in ignores would remove everything inside them anyway.
pub fn list_files(root: &Path) -> Vec<SourceFile> {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0 || !(e.file_type().is_dir() && e.file_name() == "node_modules")
        });
    let mut files = Vec::new();
    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str() else {
            continue;
        };
        if !is_typescript(name) {
            continue;
        }
        if let (Some(rel), Some(abs)) = (relative(root, entry.path()), slash(entry.path())) {
            files.push(SourceFile { rel, abs });
        }
    }
    files.sort_by(|a, b| a.rel.as_bytes().cmp(b.rel.as_bytes()));
    files
}
