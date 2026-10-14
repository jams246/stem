//! Applying tsconfig `files`, `include` and `exclude` to the listed files.

use std::collections::HashSet;
use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use super::paths::{lexical, slash};
use super::tsconfig::{Patterns, TsProject};
use super::walk::SourceFile;
use crate::error::StemError;

const DEFAULT_EXCLUDES: [&str; 3] = ["node_modules", "bower_components", "jspm_packages"];

/// Absolute glob for `pattern` relative to `base`.
fn absolute_pattern(base: &Path, pattern: &str) -> Option<String> {
    if Path::new(pattern).is_absolute() {
        return slash(&lexical(Path::new(pattern)));
    }
    let mut base = base.to_path_buf();
    let mut rest = pattern;
    loop {
        if let Some(r) = rest.strip_prefix("./") {
            rest = r;
        } else if let Some(r) = rest.strip_prefix("../") {
            base = lexical(&base.join(".."));
            rest = r;
        } else {
            break;
        }
    }
    let base = globset::escape(&slash(&base)?);
    let rest = rest.trim_end_matches('/');
    if rest.is_empty() || rest == "." {
        Some(base)
    } else {
        Some(format!("{base}/{rest}"))
    }
}

/// Whether the last segment names a folder (no wildcard and no extension).
fn names_folder(pattern: &str) -> bool {
    let last = pattern
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("");
    !last.contains(['*', '?', '.']) || last == "." || last == ".."
}

fn build(globs: &[String], file: &Path) -> Result<GlobSet, StemError> {
    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        let compiled = GlobBuilder::new(glob)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map_err(|e| StemError::Tsconfig {
                file: super::paths::shown(file),
                message: format!("invalid pattern \"{glob}\": {}", e.kind()),
            })?;
        builder.add(compiled);
    }
    builder.build().map_err(|e| StemError::Tsconfig {
        file: super::paths::shown(file),
        message: e.kind().to_string(),
    })
}

fn include_globs(patterns: &Patterns) -> Vec<String> {
    patterns
        .0
        .iter()
        .filter_map(|(base, p)| {
            let abs = absolute_pattern(base, p)?;
            Some(if names_folder(p) {
                format!("{abs}/**/*")
            } else {
                abs
            })
        })
        .collect()
}

fn exclude_globs(project: &TsProject) -> Vec<String> {
    let patterns = project.exclude.clone().unwrap_or_else(|| {
        let defaults = DEFAULT_EXCLUDES
            .iter()
            .map(|s| (project.dir.clone(), s.to_string()));
        Patterns(defaults.collect())
    });
    let mut globs = Vec::new();
    for (base, p) in &patterns.0 {
        if let Some(abs) = absolute_pattern(base, p) {
            globs.push(format!("{abs}/**"));
            globs.push(abs);
        }
    }
    if project.exclude.is_none()
        && let Some(out) = project.out_dir.as_deref().and_then(slash)
    {
        let out = globset::escape(&out);
        globs.push(format!("{out}/**"));
    }
    globs
}

/// Marks the files selected by one tsconfig project.
pub fn select(
    project: &TsProject,
    config_path: &Path,
    files: &[SourceFile],
    selected: &mut [bool],
) -> Result<(), StemError> {
    let explicit: HashSet<String> = project
        .files
        .iter()
        .flatten()
        .filter_map(|p| slash(p))
        .collect();
    let include = match (&project.include, &project.files) {
        (Some(patterns), _) => include_globs(patterns),
        (None, Some(_)) => Vec::new(),
        (None, None) => include_globs(&Patterns(vec![(project.dir.clone(), "**/*".to_string())])),
    };
    let include = build(&include, config_path)?;
    let exclude = build(&exclude_globs(project), config_path)?;
    for (file, flag) in files.iter().zip(selected.iter_mut()) {
        if explicit.contains(&file.abs)
            || (include.is_match(&file.abs) && !exclude.is_match(&file.abs))
        {
            *flag = true;
        }
    }
    Ok(())
}
