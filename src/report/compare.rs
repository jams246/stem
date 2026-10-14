//! Comparing a scan with its baseline.

use std::cmp::Ordering;

use super::entry::entry;
pub use super::entry::{Entry, MetricChange, Status};
use crate::hints::hint;
use crate::metric::Thresholds;
use crate::scan::{ParseError, Scan};
use crate::score::{Summary, subset_score10, summarize, tenths};
use crate::snapshot::Baseline;

/// Note added when the config file differs from the baseline's config.
pub const CONFIG_CHANGED_NOTE: &str =
    "The config changed since the baseline; run stem scan again to adopt it.";

/// The full comparison.
pub struct Comparison<'a> {
    pub thresholds: Thresholds,
    pub changed_before: Option<u32>,
    pub changed_after: Option<u32>,
    pub changed: usize,
    pub added: usize,
    pub removed: usize,
    pub before: Summary,
    pub after: Summary,
    pub harder: Vec<Entry<'a>>,
    pub easier: Vec<Entry<'a>>,
    pub reasons: Vec<String>,
    pub config_changed: bool,
    pub notes: Vec<String>,
    pub parse_errors: &'a [ParseError],
}

impl Comparison<'_> {
    pub fn regression(&self) -> bool {
        !self.reasons.is_empty()
    }
}

/// Compares `current` (scanned with the baseline's config) with `baseline`.
pub fn compare<'a>(
    baseline: &'a Baseline,
    current: &'a Scan,
    config_changed: bool,
) -> Comparison<'a> {
    let t = baseline.config.thresholds;
    let (mut harder, mut easier) = (Vec::new(), Vec::new());
    let (mut subset_before, mut subset_after) = (Vec::new(), Vec::new());
    let (mut changed, mut added, mut removed) = (0, 0, 0);
    let (old, new) = (&baseline.functions, &current.functions);
    let (mut i, mut j) = (0, 0);
    while i < old.len() || j < new.len() {
        let order = match (old.get(i), new.get(j)) {
            (Some(a), Some(b)) => a.id.as_bytes().cmp(b.id.as_bytes()),
            (Some(_), None) => Ordering::Less,
            _ => Ordering::Greater,
        };
        match order {
            Ordering::Less => {
                removed += 1;
                subset_before.push(&old[i]);
                i += 1;
            }
            Ordering::Greater => {
                added += 1;
                subset_after.push(&new[j]);
                let e = entry(None, &new[j], &t);
                if !e.changes.is_empty() {
                    harder.push(e);
                }
                j += 1;
            }
            Ordering::Equal => {
                if old[i].hash != new[j].hash {
                    changed += 1;
                    subset_before.push(&old[i]);
                    subset_after.push(&new[j]);
                    let e = entry(Some(&old[i]), &new[j], &t);
                    let rose = e
                        .changes
                        .iter()
                        .any(|c| c.before.is_some_and(|b| c.after > b));
                    if rose {
                        harder.push(e);
                    } else if !e.changes.is_empty() {
                        easier.push(e);
                    }
                }
                i += 1;
                j += 1;
            }
        }
    }
    for e in &mut harder {
        e.hint = hint(e.function, &t);
    }
    harder.sort_by(|a, b| {
        b.regression
            .cmp(&a.regression)
            .then(b.highest_band().cmp(&a.highest_band()))
            .then_with(|| a.function.id.as_bytes().cmp(b.function.id.as_bytes()))
    });
    easier.sort_by(|a, b| {
        b.largest_drop()
            .cmp(&a.largest_drop())
            .then_with(|| a.function.id.as_bytes().cmp(b.function.id.as_bytes()))
    });
    let changed_before = subset_score10(subset_before, &t);
    let changed_after = subset_score10(subset_after, &t);
    let mut reasons: Vec<String> = harder.iter().flat_map(|e| e.reasons.clone()).collect();
    if let (Some(b), Some(a)) = (changed_before, changed_after)
        && a < b
    {
        reasons.push(format!(
            "The changed-code score fell from {} to {}.",
            tenths(b),
            tenths(a)
        ));
    }
    let mut notes = Vec::new();
    if config_changed {
        notes.push(CONFIG_CHANGED_NOTE.to_string());
    }
    notes.extend(current.notes.iter().cloned());
    Comparison {
        thresholds: t,
        changed_before,
        changed_after,
        changed,
        added,
        removed,
        before: summarize(old, &t),
        after: summarize(new, &t),
        harder,
        easier,
        reasons,
        config_changed,
        notes,
        parse_errors: &current.parse_errors,
    }
}
