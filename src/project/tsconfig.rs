//! Reading tsconfig files, following `extends` chains.

use std::fs;
use std::path::{Path, PathBuf};

use oxc_resolver::{ResolveOptions, Resolver};
use serde_json::Value;

use super::paths::{lexical, shown};
use crate::error::StemError;
use crate::jsonc;

/// Patterns, each with the folder it is relative to.
#[derive(Clone, Debug)]
pub struct Patterns(pub Vec<(PathBuf, String)>);

const CONFIG_DIR: &str = "${configDir}";

/// Splits a path-valued setting into its base folder and the rest. A leading
/// `${configDir}` means the folder of the leaf config, as in TypeScript 5.5.
fn expand<'v>(dir: &Path, leaf: &Path, value: &'v str) -> (PathBuf, &'v str) {
    match value.strip_prefix(CONFIG_DIR) {
        Some(rest) => (leaf.to_path_buf(), rest.trim_start_matches(['/', '\\'])),
        None => (dir.to_path_buf(), value),
    }
}

/// Absolute path of a path-valued setting.
fn resolve(dir: &Path, leaf: &Path, value: &str) -> PathBuf {
    let (base, rest) = expand(dir, leaf, value);
    lexical(&base.join(rest))
}

/// The file-selection settings of one tsconfig after `extends` is applied.
#[derive(Clone, Debug, Default)]
pub struct TsProject {
    /// Folder holding the tsconfig file.
    pub dir: PathBuf,
    pub files: Option<Vec<PathBuf>>,
    pub include: Option<Patterns>,
    pub exclude: Option<Patterns>,
    pub out_dir: Option<PathBuf>,
    /// Referenced tsconfig files, absolute.
    pub references: Vec<PathBuf>,
}

fn invalid(path: &Path, message: impl Into<String>) -> StemError {
    StemError::Tsconfig {
        file: shown(path),
        message: message.into(),
    }
}

fn strings(value: &Value, path: &Path, key: &str) -> Result<Vec<String>, StemError> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .ok_or_else(|| invalid(path, format!("\"{key}\" must be a list of strings")))
}

fn resolve_extends(dir: &Path, spec: &str, from: &Path) -> Result<PathBuf, StemError> {
    let relative =
        spec.starts_with("./") || spec.starts_with("../") || Path::new(spec).is_absolute();
    if relative {
        let path = lexical(&dir.join(spec));
        let with_json = PathBuf::from(format!("{}.json", path.display()));
        return [path.clone(), with_json, path.join("tsconfig.json")]
            .into_iter()
            .find(|p| p.is_file())
            .ok_or_else(|| invalid(from, format!("cannot find extended config \"{spec}\"")));
    }
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".json".into()],
        main_fields: vec!["tsconfig".into()],
        main_files: vec!["tsconfig".into()],
        ..ResolveOptions::default()
    });
    resolver
        .resolve(dir, spec)
        .or_else(|_| resolver.resolve(dir, &format!("{spec}/tsconfig.json")))
        .map(|resolution| resolution.into_path_buf())
        .map_err(|e| {
            invalid(
                from,
                format!("cannot resolve extended config \"{spec}\": {e}"),
            )
        })
}

fn merge(base: TsProject, own: TsProject) -> TsProject {
    TsProject {
        dir: own.dir,
        files: own.files.or(base.files),
        include: own.include.or(base.include),
        exclude: own.exclude.or(base.exclude),
        out_dir: own.out_dir.or(base.out_dir),
        references: own.references,
    }
}

fn read_own(path: &Path, value: &Value, leaf: &Path) -> Result<TsProject, StemError> {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut own = TsProject {
        dir: dir.clone(),
        ..TsProject::default()
    };
    let patterns = |key: &str| -> Result<Option<Patterns>, StemError> {
        value
            .get(key)
            .map(|v| {
                strings(v, path, key).map(|patterns| {
                    let entries = patterns.iter().map(|p| {
                        let (base, rest) = expand(&dir, leaf, p);
                        (base, rest.to_string())
                    });
                    Patterns(entries.collect())
                })
            })
            .transpose()
    };
    own.include = patterns("include")?;
    own.exclude = patterns("exclude")?;
    if let Some(files) = value.get("files") {
        let files = strings(files, path, "files")?;
        own.files = Some(files.iter().map(|f| resolve(&dir, leaf, f)).collect());
    }
    if let Some(out_dir) = value
        .pointer("/compilerOptions/outDir")
        .and_then(Value::as_str)
    {
        own.out_dir = Some(resolve(&dir, leaf, out_dir));
    }
    if let Some(references) = value.get("references").and_then(Value::as_array) {
        for reference in references {
            let Some(target) = reference.get("path").and_then(Value::as_str) else {
                return Err(invalid(path, "every reference needs a \"path\""));
            };
            let target = resolve(&dir, leaf, target);
            let target = if target.is_dir() {
                target.join("tsconfig.json")
            } else {
                target
            };
            own.references.push(target);
        }
    }
    Ok(own)
}

/// Loads `path`, applying its `extends` chain.
pub fn load(path: &Path) -> Result<TsProject, StemError> {
    let leaf = path.parent().map(Path::to_path_buf).unwrap_or_default();
    load_chain(path, &leaf, 0)
}

/// Loads one config of a chain started at `leaf`; `depth` guards against cycles.
fn load_chain(path: &Path, leaf: &Path, depth: usize) -> Result<TsProject, StemError> {
    if depth > 32 {
        return Err(invalid(path, "the extends chain is too deep or circular"));
    }
    let text = fs::read_to_string(path).map_err(|e| StemError::read(shown(path), e))?;
    let value = jsonc::parse(&text).map_err(|e| invalid(path, e))?;
    if !value.is_object() {
        return Err(invalid(path, "the config must be a JSON object"));
    }
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let extends = match value.get("extends") {
        None => Vec::new(),
        Some(Value::String(s)) => vec![s.clone()],
        Some(other) => strings(other, path, "extends")?,
    };
    let mut base = TsProject::default();
    for spec in &extends {
        let target = resolve_extends(&dir, spec, path)?;
        base = merge(base, load_chain(&target, leaf, depth + 1)?);
    }
    Ok(merge(base, read_own(path, &value, leaf)?))
}
