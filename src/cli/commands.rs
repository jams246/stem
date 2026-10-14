//! The three commands, returning their output and outcome.

use std::fs;
use std::path::{Path, PathBuf};

use super::Outcome;
use crate::config;
use crate::error::StemError;
use crate::explain;
use crate::project::{self, paths};
use crate::report::compare::compare as compare_scans;
use crate::report::{compare_json, compare_text, scan_json, scan_text};
use crate::scan::scan as scan_project;
use crate::snapshot::{read_baseline, snapshot_json};

type CommandResult = Result<(String, Outcome), StemError>;

fn write_snapshot(path: &Path, text: &str) -> Result<(), StemError> {
    let shown = paths::shown(path);
    let fail = |source| StemError::Write {
        path: shown.clone(),
        source,
    };
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(fail)?;
    }
    fs::write(path, text).map_err(fail)
}

pub fn scan(
    dir: Option<PathBuf>,
    output: Option<PathBuf>,
    json: bool,
    ignore: &[String],
) -> CommandResult {
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let project = project::discover(&dir)?;
    let config = config::load(&project.root)?
        .unwrap_or_default()
        .with_ignores(ignore);
    let scan = scan_project(&project, &config)?;
    if let Some(output) = output {
        write_snapshot(&output, &snapshot_json(&scan).render())?;
    }
    let text = if json {
        scan_json(&scan).render()
    } else {
        scan_text(&scan)
    };
    Ok((text, Outcome::Pass))
}

pub fn compare(
    baseline: &Path,
    dir: Option<PathBuf>,
    json: bool,
    ignore: &[String],
) -> CommandResult {
    if !ignore.is_empty() {
        return Err(StemError::Usage(
            "compare does not accept --ignore; it always uses the config stored in the baseline"
                .to_string(),
        ));
    }
    let shown = baseline.display().to_string();
    // Reports name only the file, so output never holds an absolute path and
    // does not depend on how the path was spelled.
    let name = baseline
        .file_name()
        .map_or_else(|| shown.clone(), |n| n.to_string_lossy().into_owned());
    let baseline = read_baseline(baseline, &shown)?;
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let project = project::discover(&dir)?;
    let current_file = config::load(&project.root)?.unwrap_or_default();
    // Blind spot: a pattern removed from the file is missed, since flags and file patterns look alike.
    let config_changed = current_file.thresholds != baseline.config.thresholds
        || current_file
            .ignore
            .iter()
            .any(|p| !baseline.config.ignore.contains(p));
    let current = scan_project(&project, &baseline.config)?;
    let comparison = compare_scans(&baseline, &current, config_changed);
    let text = if json {
        compare_json(&comparison).render()
    } else {
        compare_text(&comparison, &name)
    };
    let outcome = if comparison.regression() {
        Outcome::Regression
    } else {
        Outcome::Pass
    };
    Ok((text, outcome))
}

pub fn explain(topic: Option<&str>) -> CommandResult {
    let thresholds = match project::find_root(Path::new(".")) {
        Ok((root, _)) => config::load(&root)?.unwrap_or_default().thresholds,
        Err(_) => Default::default(),
    };
    Ok((explain::explain(topic, &thresholds)?, Outcome::Pass))
}
