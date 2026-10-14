//! Measuring a project: filter files, parse in parallel, collect functions.

use std::fs;
use std::path::PathBuf;

use rayon::prelude::*;

use crate::analyze::{FileOutcome, FunctionMetrics, analyze_source};
use crate::config::Config;
use crate::error::StemError;
use crate::ignore::Ignores;
use crate::project::{Project, paths};

/// A file that could not be measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub path: String,
    pub message: String,
}

/// Everything one scan measured.
#[derive(Clone, Debug)]
pub struct Scan {
    pub root: PathBuf,
    pub config: Config,
    pub scanned: usize,
    pub ignored: usize,
    pub parse_errors: Vec<ParseError>,
    /// Functions sorted by ID in byte order.
    pub functions: Vec<FunctionMetrics>,
    /// Informational notes, in a fixed order.
    pub notes: Vec<String>,
}

/// Note shown when a project has no functions.
pub const NO_FUNCTIONS_NOTE: &str = "No functions found.";

/// Note shown for an ignore pattern that matches no files.
pub fn unmatched_note(pattern: &str) -> String {
    format!("Ignore pattern \"{pattern}\" matches no files; check it for typos.")
}

/// Scans the project's candidate files with `config`.
pub fn scan(project: &Project, config: &Config) -> Result<Scan, StemError> {
    let ignores = Ignores::new(&config.ignore)?;
    let filtered = ignores.filter(project.candidates.clone());
    if filtered.kept.is_empty() {
        return Err(StemError::NoFiles(paths::shown(&project.root)));
    }
    let outcomes: Vec<(String, FileOutcome)> = filtered
        .kept
        .par_iter()
        .map(|rel| {
            let outcome = match fs::read(project.root.join(rel)) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(text) => analyze_source(rel, &text),
                    Err(_) => FileOutcome::ParseError("the file is not valid UTF-8".to_string()),
                },
                Err(e) => FileOutcome::ParseError(format!("cannot read the file: {e}")),
            };
            (rel.clone(), outcome)
        })
        .collect();
    let mut functions = Vec::new();
    let mut parse_errors = Vec::new();
    for (path, outcome) in outcomes {
        match outcome {
            FileOutcome::Functions(found) => functions.extend(found),
            FileOutcome::ParseError(message) => parse_errors.push(ParseError { path, message }),
        }
    }
    functions.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    parse_errors.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut notes: Vec<String> = filtered
        .unmatched
        .iter()
        .map(|p| unmatched_note(p))
        .collect();
    if functions.is_empty() {
        notes.push(NO_FUNCTIONS_NOTE.to_string());
    }
    Ok(Scan {
        root: project.root.clone(),
        config: config.clone(),
        scanned: filtered.kept.len(),
        ignored: filtered.ignored,
        parse_errors,
        functions,
        notes,
    })
}
