//! Listed functions: which metrics changed and why an entry is a regression.

use crate::analyze::FunctionMetrics;
use crate::hints::Hint;
use crate::metric::{Band, Metric, Thresholds};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Added,
    Changed,
}

impl Status {
    pub fn key(self) -> &'static str {
        match self {
            Status::Added => "added",
            Status::Changed => "changed",
        }
    }
}

/// Before and after of one relevant metric.
#[derive(Clone, Debug)]
pub struct MetricChange {
    pub metric: Metric,
    pub before: Option<u32>,
    pub band_before: Option<Band>,
    pub after: u32,
    pub band_after: Band,
}

/// A listed function.
#[derive(Clone, Debug)]
pub struct Entry<'a> {
    pub function: &'a FunctionMetrics,
    pub status: Status,
    pub changes: Vec<MetricChange>,
    pub regression: bool,
    pub reasons: Vec<String>,
    pub hint: Option<Hint>,
}

impl Entry<'_> {
    pub(super) fn highest_band(&self) -> Band {
        self.changes
            .iter()
            .map(|c| c.band_after)
            .max()
            .unwrap_or(Band::Low)
    }

    pub(super) fn largest_drop(&self) -> u32 {
        self.changes
            .iter()
            .map(|c| {
                c.band_before
                    .map_or(0, |b| b.rank().saturating_sub(c.band_after.rank()))
            })
            .max()
            .unwrap_or(0)
    }
}

fn change(
    metric: Metric,
    before: Option<&FunctionMetrics>,
    after: &FunctionMetrics,
    t: &Thresholds,
) -> MetricChange {
    let before = before.map(|f| f.value(metric));
    let after = after.value(metric);
    MetricChange {
        metric,
        before,
        band_before: before.map(|v| t.band(metric, v)),
        after,
        band_after: t.band(metric, after),
    }
}

/// Regression reason for one metric, if it is one.
fn reason(name: &str, c: &MetricChange) -> Option<String> {
    if c.band_after < Band::High {
        return None;
    }
    let (band, metric) = (c.band_after.name(), c.metric.label());
    match (c.before, c.band_before) {
        (None, _) => Some(format!(
            "{name} is new and in the {band} band for {metric}."
        )),
        (Some(_), Some(before)) if c.band_after > before => {
            Some(format!("{name} entered the {band} band for {metric}."))
        }
        (Some(before), Some(_)) if c.after > before => Some(format!(
            "{name} rose from {before} to {} for {metric}, staying in the {band} band.",
            c.after
        )),
        _ => None,
    }
}

pub(super) fn entry<'a>(
    before: Option<&FunctionMetrics>,
    after: &'a FunctionMetrics,
    t: &Thresholds,
) -> Entry<'a> {
    let changes: Vec<MetricChange> = Metric::ALL
        .into_iter()
        .map(|m| change(m, before, after, t))
        .filter(|c| match c.before {
            Some(b) => b != c.after,
            None => c.band_after > Band::Low,
        })
        .collect();
    let reasons: Vec<String> = changes
        .iter()
        .filter_map(|c| reason(after.name(), c))
        .collect();
    Entry {
        function: after,
        status: if before.is_some() {
            Status::Changed
        } else {
            Status::Added
        },
        regression: !reasons.is_empty(),
        reasons,
        changes,
        hint: None,
    }
}
