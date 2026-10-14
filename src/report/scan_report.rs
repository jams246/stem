//! `stem scan` output.

use std::fmt::Write as _;

use super::format::{LEARN_LINE, location, metric_line};
use crate::analyze::FunctionMetrics;
use crate::hints::hint;
use crate::json::Json;
use crate::metric::{Band, Metric, Thresholds};
use crate::scan::Scan;
use crate::score::{potential, summarize, tenths};
use crate::snapshot::{config_json, files_json, summary_json};
use crate::{METRICS_VERSION, SCHEMA_VERSION, STEM_VERSION};

/// Up to five functions with the most to gain, most first, ties by ID.
pub fn top_functions(scan: &Scan) -> Vec<(&FunctionMetrics, u64)> {
    let thresholds = &scan.config.thresholds;
    let mut ranked: Vec<(&FunctionMetrics, u64)> = scan
        .functions
        .iter()
        .map(|f| (f, potential(f, thresholds)))
        .filter(|(_, p)| *p > 0)
        .collect();
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.id.as_bytes().cmp(b.0.id.as_bytes()))
    });
    ranked.truncate(5);
    ranked
}

/// Metrics of a function at moderate or above, in reporting order.
fn raised_metrics(function: &FunctionMetrics, thresholds: &Thresholds) -> Vec<Metric> {
    Metric::ALL
        .into_iter()
        .filter(|&m| thresholds.band(m, function.value(m)) > Band::Low)
        .collect()
}

pub fn scan_text(scan: &Scan) -> String {
    let thresholds = &scan.config.thresholds;
    let summary = summarize(&scan.functions, thresholds);
    let mut out = String::from("stem scan\n\n");
    let score = tenths(summary.score10);
    let _ = writeln!(out, "{:<15}{score}  grade {}", "Overall", summary.grade());
    for metric in Metric::ALL {
        let score = tenths(summary.metric(metric).score10);
        let _ = writeln!(
            out,
            "  {:<22}{score:<7}{}",
            metric.label(),
            metric.description()
        );
    }
    let _ = writeln!(out, "{:<15}{}", "Functions", summary.functions);
    let files = format!("{} scanned, {} ignored", scan.scanned, scan.ignored);
    let _ = writeln!(out, "{:<15}{files}", "Files");

    let top = top_functions(scan);
    if !top.is_empty() {
        let _ = writeln!(out, "\nMost to gain ({})", top.len());
        for (function, _) in &top {
            let _ = writeln!(out, "  {}", location(function));
            for metric in raised_metrics(function, thresholds) {
                let line = metric_line(metric, None, function.value(metric), thresholds);
                let _ = writeln!(out, "    {line}");
            }
            if let Some(hint) = hint(function, thresholds) {
                let _ = writeln!(out, "    {}\n    {}", hint.found, hint.action);
            }
        }
    }
    if !scan.parse_errors.is_empty() {
        let _ = writeln!(out, "\nCould not parse ({})", scan.parse_errors.len());
        for e in &scan.parse_errors {
            let _ = writeln!(out, "  {}: {}", e.path, e.message);
        }
    }
    if !scan.notes.is_empty() {
        out.push_str("\nNotes\n");
        for note in &scan.notes {
            let _ = writeln!(out, "  {note}");
        }
    }
    let _ = writeln!(out, "\n{LEARN_LINE}");
    out
}

fn top_json(function: &FunctionMetrics, potential: u64, thresholds: &Thresholds) -> Json {
    let metrics = Metric::ALL
        .iter()
        .map(|&m| {
            let value = function.value(m);
            let band = thresholds.band(m, value);
            let entry = Json::obj([("value", Json::int(value)), ("band", Json::str(band.key()))]);
            (m.key().to_string(), entry)
        })
        .collect();
    let hint = hint(function, thresholds).map_or(Json::Null, |h| Json::str(h.text()));
    Json::obj([
        ("id", Json::str(&function.id)),
        ("path", Json::str(function.path())),
        ("name", Json::str(function.name())),
        ("line", Json::int(function.line)),
        ("potential", Json::int(potential)),
        ("metrics", Json::Obj(metrics)),
        ("hint", hint),
    ])
}

pub fn scan_json(scan: &Scan) -> Json {
    let thresholds = &scan.config.thresholds;
    let summary = summarize(&scan.functions, thresholds);
    let top = top_functions(scan)
        .into_iter()
        .map(|(f, p)| top_json(f, p, thresholds))
        .collect();
    Json::obj([
        ("schemaVersion", Json::int(SCHEMA_VERSION)),
        ("metricsVersion", Json::int(METRICS_VERSION)),
        ("stemVersion", Json::str(STEM_VERSION)),
        ("config", config_json(&scan.config)),
        ("files", files_json(scan)),
        ("summary", summary_json(&summary)),
        ("top", Json::Arr(top)),
        (
            "notes",
            Json::Arr(scan.notes.iter().map(Json::str).collect()),
        ),
    ])
}
