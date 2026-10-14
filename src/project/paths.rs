//! Path helpers: lexical normalization and `/`-separated strings.

use std::path::{Component, Path, PathBuf};

/// Resolves `.` and `..` components without touching the filesystem.
pub fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The path as an owned string, or `None` when it is not UTF-8.
pub fn slash(path: &Path) -> Option<String> {
    path.to_str().map(str::to_string)
}

/// Path of `path` relative to `root`, with `/` separators.
pub fn relative(root: &Path, path: &Path) -> Option<String> {
    let rest = path.strip_prefix(root).ok()?;
    let parts: Option<Vec<&str>> = rest.components().map(|c| c.as_os_str().to_str()).collect();
    Some(parts?.join("/"))
}

/// Path for messages: relative to the current folder when possible, so
/// output never contains absolute paths from outside the user's view.
pub fn shown(path: &Path) -> String {
    let relative = std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(&cwd).ok().map(Path::to_path_buf));
    match relative {
        Some(rel) if rel.as_os_str().is_empty() => ".".to_string(),
        Some(rel) => slash(&rel).unwrap_or_else(|| rel.display().to_string()),
        None => path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
    }
}

/// Whether the file name is a TypeScript source file stem measures.
pub fn is_typescript(name: &str) -> bool {
    let declaration = [".d.ts", ".d.mts", ".d.cts"]
        .iter()
        .any(|ext| name.ends_with(ext));
    let source = [".ts", ".tsx", ".mts", ".cts"]
        .iter()
        .any(|ext| name.ends_with(ext));
    source && !declaration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_should_resolve_parent_components() {
        assert_eq!(lexical(Path::new("/a/b/../c/./d")), PathBuf::from("/a/c/d"));
    }

    #[test]
    fn is_typescript_should_skip_declarations_and_javascript() {
        let names = [
            "a.ts", "a.tsx", "a.mts", "a.cts", "a.d.ts", "a.d.mts", "a.js",
        ];
        assert_eq!(
            names.map(is_typescript),
            [true, true, true, true, false, false, false]
        );
    }
}
