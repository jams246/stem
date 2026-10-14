//! Integer scoring in tenths of a point.

use crate::analyze::FunctionMetrics;
use crate::metric::{Metric, Thresholds};

/// Score and weighted band totals of one metric.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetricSummary {
    /// Score in tenths, 0 to 1000.
    pub score10: u32,
    /// Weighted totals per band, in [`crate::metric::Band::ALL`] order.
    pub lines: [u64; 4],
}

/// Scores of a set of functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Overall score in tenths.
    pub score10: u32,
    pub functions: usize,
    /// Per metric, in [`Metric::ALL`] order.
    pub metrics: [MetricSummary; 4],
}

impl Summary {
    pub fn grade(&self) -> &'static str {
        grade(self.score10)
    }

    pub fn metric(&self, metric: Metric) -> &MetricSummary {
        &self.metrics[metric.index()]
    }
}

/// `max(0, 1000 - ceil(500 (Wm + 4 Wh + 8 Wv) / Wt))`, all in integers.
pub fn metric_score10(lines: [u64; 4]) -> u32 {
    let total: u64 = lines.iter().sum();
    if total == 0 {
        return 1000;
    }
    let weighted = u128::from(lines[1]) + 4 * u128::from(lines[2]) + 8 * u128::from(lines[3]);
    let numerator = 500 * weighted;
    let total = u128::from(total);
    let penalty = numerator.div_ceil(total);
    u32::try_from(1000u128.saturating_sub(penalty)).unwrap_or(0)
}

/// Average of the four metric scores, rounded half up to tenths.
pub fn overall_score10(scores: [u32; 4]) -> u32 {
    (scores.iter().sum::<u32>() + 2) / 4
}

/// Letter grade for a score in tenths.
pub fn grade(score10: u32) -> &'static str {
    match score10 {
        850.. => "A",
        700.. => "B",
        550.. => "C",
        250.. => "D",
        _ => "E",
    }
}

/// Scores `functions` with `thresholds`.
pub fn summarize<'a>(
    functions: impl IntoIterator<Item = &'a FunctionMetrics>,
    thresholds: &Thresholds,
) -> Summary {
    let mut lines = [[0u64; 4]; 4];
    let mut count = 0;
    for function in functions {
        count += 1;
        let weight = function.weight();
        for metric in Metric::ALL {
            let band = thresholds.band(metric, function.value(metric));
            lines[metric.index()][band as usize] += weight;
        }
    }
    let metrics = lines.map(|lines| MetricSummary {
        score10: metric_score10(lines),
        lines,
    });
    Summary {
        score10: overall_score10(metrics.map(|m| m.score10)),
        functions: count,
        metrics,
    }
}

/// Overall score of a subset, or `None` when it is empty.
pub fn subset_score10<'a>(
    functions: impl IntoIterator<Item = &'a FunctionMetrics>,
    thresholds: &Thresholds,
) -> Option<u32> {
    let summary = summarize(functions, thresholds);
    (summary.functions > 0).then_some(summary.score10)
}

/// Potential gain of a function: weight times the sum of its band costs.
pub fn potential(function: &FunctionMetrics, thresholds: &Thresholds) -> u64 {
    let cost: u64 = Metric::ALL
        .iter()
        .map(|&m| thresholds.band(m, function.value(m)).cost())
        .sum();
    function.weight() * cost
}

/// Formats tenths with one decimal place, e.g. `724` as `72.4`.
pub fn tenths(value: u32) -> String {
    format!("{}.{}", value / 10, value % 10)
}

/// Formats a signed difference of tenths, e.g. `+0.7` or `-1.2`.
pub fn signed_tenths(after: u32, before: u32) -> String {
    let (sign, diff) = if after >= before {
        ('+', after - before)
    } else {
        ('-', before - after)
    };
    format!("{sign}{}", tenths(diff))
}
