//! Shared text formatting: metric lines, locations and aligned rows.

use crate::analyze::FunctionMetrics;
use crate::metric::{Metric, Thresholds};

/// Last line of every text report.
pub const LEARN_LINE: &str = "Learn what a metric measures: stem explain <metric>";

/// A self-explanatory metric line: name, value, band, target, description.
///
/// With `before`, the value is shown as `before -> after`.
pub fn metric_line(
    metric: Metric,
    before: Option<u32>,
    after: u32,
    thresholds: &Thresholds,
) -> String {
    let band = thresholds.band(metric, after);
    let value = match before {
        Some(before) => format!("{before} -> {after}"),
        None => after.to_string(),
    };
    let band_text = match thresholds.target(metric, band) {
        Some(target) => format!("{}, aim for {target} or less", band.name()),
        None => band.name().to_string(),
    };
    format!(
        "{} {value} ({band_text}): {}",
        metric.label(),
        metric.description()
    )
}

/// `name  path:line` heading of a listed function.
pub fn location(function: &FunctionMetrics) -> String {
    format!("{}  {}:{}", function.name(), function.path(), function.line)
}

/// Source evidence shown whenever a positive usage gap is listed.
pub fn usage_gap_location(function: &FunctionMetrics) -> Option<String> {
    function.usage_gap_evidence.as_ref().map(|e| {
        format!(
            "{}: references at lines {} and {}; declared at line {}.",
            e.name, e.from_line, e.to_line, e.declaration_line,
        )
    })
}

/// Pads `text` to `width`, keeping at least two spaces before the next column.
pub fn column(text: &str, width: usize) -> String {
    let width = width.max(text.len() + 2);
    format!("{text:<width$}")
}

/// Aligned summary row: label, values, change, then the rest.
pub fn row(label: &str, values: &str, change: &str, rest: &str) -> String {
    let line = format!(
        "{}{}{}{rest}",
        column(label, 15),
        column(values, 14),
        column(change, 10)
    );
    line.trim_end().to_string()
}
