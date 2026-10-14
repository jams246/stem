//! Building snapshot JSON.

use crate::analyze::FunctionMetrics;
use crate::config::Config;
use crate::json::Json;
use crate::metric::{Band, Metric};
use crate::scan::{ParseError, Scan};
use crate::score::{Summary, summarize};
use crate::{METRICS_VERSION, SCHEMA_VERSION, STEM_VERSION};

/// The effective config: sorted ignore patterns and all thresholds.
pub fn config_json(config: &Config) -> Json {
    let thresholds = Metric::ALL
        .iter()
        .map(|&m| {
            let values = config.thresholds.get(m).map(Json::int).to_vec();
            (m.key().to_string(), Json::Arr(values))
        })
        .collect();
    Json::obj([
        (
            "ignore",
            Json::Arr(config.ignore.iter().map(Json::str).collect()),
        ),
        ("thresholds", Json::Obj(thresholds)),
    ])
}

pub fn parse_errors_json(errors: &[ParseError]) -> Json {
    Json::Arr(
        errors
            .iter()
            .map(|e| {
                Json::obj([
                    ("path", Json::str(&e.path)),
                    ("message", Json::str(&e.message)),
                ])
            })
            .collect(),
    )
}

pub fn files_json(scan: &Scan) -> Json {
    Json::obj([
        ("scanned", Json::int(scan.scanned)),
        ("ignored", Json::int(scan.ignored)),
        ("parseErrors", parse_errors_json(&scan.parse_errors)),
    ])
}

pub fn summary_json(summary: &Summary) -> Json {
    let metrics = Metric::ALL
        .iter()
        .map(|&m| {
            let ms = summary.metric(m);
            let lines = Band::ALL
                .iter()
                .map(|&b| (b.key().to_string(), Json::int(ms.lines[b as usize])))
                .collect();
            let value = Json::obj([
                ("score", Json::tenths(ms.score10)),
                ("lines", Json::Obj(lines)),
            ]);
            (m.key().to_string(), value)
        })
        .collect();
    Json::obj([
        ("score", Json::tenths(summary.score10)),
        ("grade", Json::str(summary.grade())),
        ("functions", Json::int(summary.functions)),
        ("metrics", Json::Obj(metrics)),
    ])
}

pub fn function_json(function: &FunctionMetrics) -> Json {
    let mut pairs = vec![
        ("id".to_string(), Json::str(&function.id)),
        ("line".to_string(), Json::int(function.line)),
    ];
    for metric in Metric::ALL {
        pairs.push((metric.key().to_string(), Json::int(function.value(metric))));
    }
    let increments = function
        .increments
        .iter()
        .map(|i| {
            Json::obj([
                ("line", Json::int(i.line)),
                ("kind", Json::str(&i.kind)),
                ("base", Json::int(i.base)),
                ("nesting", Json::int(i.nesting)),
            ])
        })
        .collect();
    pairs.push(("increments".to_string(), Json::Arr(increments)));
    pairs.push(("hash".to_string(), Json::str(&function.hash)));
    Json::Obj(pairs)
}

/// The whole snapshot.
pub fn snapshot_json(scan: &Scan) -> Json {
    let summary = summarize(&scan.functions, &scan.config.thresholds);
    Json::obj([
        ("schemaVersion", Json::int(SCHEMA_VERSION)),
        ("metricsVersion", Json::int(METRICS_VERSION)),
        ("stemVersion", Json::str(STEM_VERSION)),
        ("config", config_json(&scan.config)),
        ("files", files_json(scan)),
        ("summary", summary_json(&summary)),
        (
            "functions",
            Json::Arr(scan.functions.iter().map(function_json).collect()),
        ),
    ])
}
